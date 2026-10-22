import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:winterland_app/src/rust/frb_generated.dart';

import 'app.dart';

Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final defaultFlakePath = Directory.current.absolute.path;
  runApp(WinterlandApp(defaultFlakePath));
}
