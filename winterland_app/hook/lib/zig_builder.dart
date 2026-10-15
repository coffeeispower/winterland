import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:hooks/hooks.dart';
import 'package:logging/logging.dart';

/// Builds the winterland_shovel Zig package as a code asset.
///
/// Flutter uses the dynamic artifact by default. Native consumers can use the
/// static artifact produced by the same Zig project with `zig build`. The
/// asset link mode follows the preference requested by the invoker.
class ZigBuilder implements Builder {
  final String name;
  final String assetName;
  final String? packageName;
  final String sourceDirectory;
  final ZigTarget target;
  final String zigExecutable;
  final List<String> flags;

  ZigBuilder.library({
    required this.name,
    required this.assetName,
    required this.target,
    this.packageName,
    this.sourceDirectory = '.',
    this.zigExecutable = 'zig',
    this.flags = const [],
  });

  @override
  Future<void> run({
    required BuildInput input,
    required BuildOutputBuilder output,
    Logger? logger,
    List<AssetRouting> routing = const [ToAppBundle()],
  }) async {
    logger ??= Logger('ZigBuilder');
    if (!input.config.buildCodeAssets) {
      logger.info('Code assets are disabled; skipping $assetName.');
      return;
    }

    final sourceUri = input.packageRoot.resolve(
      sourceDirectory.endsWith('/') ? sourceDirectory : '$sourceDirectory/',
    );
    final sourceDir = Directory.fromUri(sourceUri);
    if (!await sourceDir.exists()) {
      throw StateError('Zig source directory does not exist: $sourceUri');
    }

    final preference = input.config.code.linkModePreference;
    final linkMode = switch (preference) {
      LinkModePreference.static || LinkModePreference.preferStatic =>
        StaticLinking(),
      LinkModePreference.dynamic || LinkModePreference.preferDynamic =>
        DynamicLoadingBundled(),
      _ => DynamicLoadingBundled(),
    };
    final libraryName = input.config.code.targetOS.libraryFileName(
      name,
      linkMode,
    );
    final outDir = Directory.fromUri(input.outputDirectory);
    await outDir.create(recursive: true);
    final libraryUri = input.outputDirectory.resolve('lib/$libraryName');

    final arguments = <String>[
      'build',
      '--prefix',
      input.outputDirectory.toFilePath(),
      '-Dtarget=${target.triple}',
      '-Doptimize=${target.optimize}',
      ...flags,
    ];
    logger.info('Building Zig library $name for ${input.config.code.targetOS}...');

    final result = await Process.run(
      zigExecutable,
      arguments,
      workingDirectory: sourceUri.toFilePath(),
      runInShell: false,
      stdoutEncoding: systemEncoding,
      stderrEncoding: systemEncoding,
      includeParentEnvironment: true,
      environment: {'LANG': 'C.UTF-8', ...Platform.environment},
    );
    if (result.stdout.toString().trim().isNotEmpty) {
      logger.info(result.stdout.toString().trimRight());
    }
    if (result.stderr.toString().trim().isNotEmpty) {
      logger.info(result.stderr.toString().trimRight());
    }
    if (result.exitCode != 0) {
      throw ProcessException(
        zigExecutable,
        arguments,
        'Zig compilation failed with exit code ${result.exitCode}.\n'
            '${result.stdout}\n${result.stderr}',
        result.exitCode,
      );
    }

    if (!await File.fromUri(libraryUri).exists()) {
      throw StateError(
        'Zig exited successfully but did not produce the expected library: '
        '$libraryUri',
      );
    }
    for (final route in routing) {
      output.assets.code.add(
        CodeAsset(
          package: packageName ?? input.packageName,
          name: assetName,
          file: libraryUri,
          linkMode: linkMode,
        ),
        routing: route,
      );
    }

    output.dependencies.addAll(
      await sourceDir
          .list(recursive: true, followLinks: false)
          .where((entity) =>
              entity is File &&
              (entity.path.endsWith('.zig') || entity.path.endsWith('.zon')))
          .map((entity) => entity.uri)
          .toList(),
    );
    logger.info('Built $libraryUri');
  }
}

class ZigTarget {
  final String triple;
  final String optimize;

  const ZigTarget({required this.triple, this.optimize = 'ReleaseSafe'});
}

ZigTarget resolveZigTarget(CodeConfig config) {
  final arch = switch (config.targetArchitecture) {
    Architecture.arm64 => 'aarch64',
    Architecture.x64 => 'x86_64',
    _ => throw UnsupportedError(
        'Architecture not supported by winterland_shovel: '
        '${config.targetArchitecture.name}',
      ),
  };
  return switch (config.targetOS) {
    OS.linux => ZigTarget(triple: '$arch-linux-gnu'),
    OS.macOS => ZigTarget(triple: '$arch-macos'),
    OS.windows => ZigTarget(triple: '$arch-windows-gnu'),
    OS.android => ZigTarget(triple: '$arch-linux-android'),
    OS.iOS => ZigTarget(triple: '$arch-ios'),
    _ => throw UnsupportedError(
        'Operating system not supported by winterland_shovel: '
        '${config.targetOS.name}',
      ),
  };
}
