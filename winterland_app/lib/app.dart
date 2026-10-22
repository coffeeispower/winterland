import 'package:flutter/widgets.dart';

import 'design/app_skeleton.dart';
import 'pages/flake_workspace_page.dart';
import 'pages/open_flake_page.dart';

class WinterlandApp extends StatelessWidget {
  final String defaultFlakePath;

  const WinterlandApp(this.defaultFlakePath, {super.key});

  @override
  Widget build(BuildContext context) {
    return WinterlandAppSkel(
      "/",
      routes: {
        '/': (_) => OpenFlakePage(defaultFlakePath: defaultFlakePath),
        '/workspace': (_) => const FlakeWorkspacePage(),
      },
    );
  }
}
