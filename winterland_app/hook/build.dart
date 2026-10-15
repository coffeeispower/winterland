import 'package:hooks/hooks.dart';

import 'lib/odin_builder.dart';

import 'package:code_assets/code_assets.dart';

void main(List<String> arguments) async {
  await build(arguments, (input, output) async {
    await OdinBuilder.library(
      name: 'winterland_shovel',
      assetName: 'winterland_shovel.dart',
      sourceDirectory: '../winterland_shovel/',
      target: resolveOdinTarget(input.config.code),
      odinExecutable: String.fromEnvironment(
        "ODIN_EXECUTABLE",
        defaultValue: "odin",
      ),
    ).run(input: input, output: output);
  });
}
