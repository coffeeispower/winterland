import 'package:hooks/hooks.dart';

import 'lib/zig_builder.dart';

import 'package:code_assets/code_assets.dart';

void main(List<String> arguments) async {
  await build(arguments, (input, output) async {
    await ZigBuilder.library(
      name: 'winterland_shovel',
      assetName: 'winterland_shovel.dart',
      sourceDirectory: '../winterland_shovel/',
      target: resolveZigTarget(input.config.code),
      zigExecutable: String.fromEnvironment(
        'ZIG_EXECUTABLE',
        defaultValue: 'zig',
      ),
    ).run(input: input, output: output);
  });
}
