import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:hooks/hooks.dart';
import 'package:logging/logging.dart';

/// Builds an Odin package as a native dynamic library.
///
/// The Odin compiler must be available in PATH, unless [odinExecutable]
/// specifies another executable.
class OdinBuilder implements Builder {
  final String name;
  final String assetName;
  final String? packageName;

  /// Directory containing the Odin package, relative to packageRoot.
  final String sourceDirectory;

  /// Odin target, for example "linux_amd64" or "linux_arm64".
  ///
  /// Leave null to use the compiler's default target.
  final OdinTarget? target;

  final String odinExecutable;
  final List<String> flags;

  OdinBuilder.library({
    required this.name,
    required this.assetName,
    this.packageName,
    this.sourceDirectory = '.',
    this.target,
    this.odinExecutable = "odin",
    this.flags = const [],
  });

  @override
  Future<void> run({
    required BuildInput input,
    required BuildOutputBuilder output,
    Logger? logger,
    List<AssetRouting> routing = const [ToAppBundle()],
  }) async {
    logger ??= Logger('OdinBuilder');

    if (!input.config.buildCodeAssets) {
      logger.info('Code assets are disabled; skipping $assetName.');
      return;
    }

    final packageRoot = input.packageRoot;
    final outDir = input.outputDirectory;

    await Directory.fromUri(outDir).create(recursive: true);

    final sourceUri = packageRoot.resolve(
      sourceDirectory.endsWith('/') ? sourceDirectory : '$sourceDirectory/',
    );

    final sourceDir = Directory.fromUri(sourceUri);

    if (!await sourceDir.exists()) {
      throw StateError('Odin source directory does not exist: $sourceUri');
    }

    // This first implementation always builds a bundled dynamic library.
    final linkMode = DynamicLoadingBundled();

    final libraryUri = outDir.resolve(
      input.config.code.targetOS.libraryFileName(name, linkMode),
    );

    final arguments = <String>[
      'build',
      sourceUri.toFilePath(),
      '-build-mode:dll',
      '-out:${libraryUri.toFilePath()}',
      ...(target?.arguments ?? <String>[]),
      ...flags,
    ];
    logger.info(
      'Building Odin library $name for '
      '${input.config.code.targetOS}...',
    );

    final result = await Process.run(
      odinExecutable,
      arguments,
      workingDirectory: packageRoot.toFilePath(),
      runInShell: false,

      stdoutEncoding: systemEncoding,
      stderrEncoding: systemEncoding,
      includeParentEnvironment: true,
      environment: {'LANG': "C.UTF-8", ...Platform.environment},
    );

    if (result.stdout.toString().isNotEmpty) {
      logger.info(result.stdout.toString().trimRight());
    }

    if (result.stderr.toString().isNotEmpty) {
      logger.info(result.stderr.toString().trimRight());
    }

    if (result.exitCode != 0) {
      throw ProcessException(
        odinExecutable,
        arguments,
        'Odin compilation failed with exit code '
        '${result.exitCode}.\n'
        '${result.stdout}\n${result.stderr}',
        result.exitCode,
      );
    }
    final libraryPath = File.fromUri(libraryUri);
    if (!await libraryPath.exists()) {
      throw StateError(
        'Odin exited successfully but did not produce '
        'the expected library: $libraryUri',
      );
    }
    await Process.run("chmod", [
      "-x",
      libraryUri.toFilePath(),
    ], runInShell: false);
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

    // Track Odin source files as build dependencies.
    final sourceFiles = await sourceDir
        .list(recursive: true, followLinks: false)
        .where((entity) => entity is File && entity.path.endsWith('.odin'))
        .map((entity) => entity.uri)
        .toList();

    output.dependencies.addAll(sourceFiles);

    logger.info('Built $libraryUri');
  }
}

class OdinTarget {
  final String target;
  final List<String> extraArguments;

  const OdinTarget({required this.target, this.extraArguments = const []});

  List<String> get arguments => ['-target:$target', ...extraArguments];
}

OdinTarget resolveOdinTarget(CodeConfig config) {
  final os = config.targetOS;
  final architecture = config.targetArchitecture;

  final arch = switch (architecture) {
    Architecture.arm64 => 'arm64',
    Architecture.x64 => 'amd64',
    _ => throw UnsupportedError(
      'Arquitetura não suportada pelo Shovel: '
      '${architecture.name}',
    ),
  };

  return switch (os) {
    OS.linux => OdinTarget(target: 'linux_$arch'),
    OS.macOS => OdinTarget(target: 'darwin_$arch'),
    OS.windows => OdinTarget(target: 'windows_$arch'),

    // O target base existe, mas estes subtargets exigem
    // configurar corretamente o SDK e o linker da plataforma.
    OS.android => OdinTarget(
      target: 'linux_$arch',
      extraArguments: const ['-subtarget:android'],
    ),
    OS.iOS => OdinTarget(
      target: 'darwin_$arch',
      extraArguments: const ['-subtarget:ios'],
    ),

    _ => throw UnsupportedError(
      'Sistema operativo não suportado pelo Shovel: ${os.name}',
    ),
  };
}
