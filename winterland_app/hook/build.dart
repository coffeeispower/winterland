import 'package:flutter_rust_bridge_hooks/flutter_rust_bridge_hooks.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    await const FlutterRustBridgeNativeAssetsBuilder(
      cratePath: '../winterland_shovel',
      features: <String>['flutter_rust_bridge'],
    ).run(input: input, output: output);
  });
}
