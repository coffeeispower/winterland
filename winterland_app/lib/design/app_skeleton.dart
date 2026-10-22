import 'package:flutter/widgets.dart';

import 'theme.dart';
import 'page_route.dart';

class WinterlandAppSkel extends StatelessWidget {
  final String initialRoute;
  final Map<String, WidgetBuilder> routes;
  final ThemeData theme;

  const WinterlandAppSkel(
    this.initialRoute, {
    this.routes = const {},
    this.theme = const ThemeData(),
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return Theme(
      data: theme,
      child: WidgetsApp(
        color: Color(0xFF000000),
        initialRoute: initialRoute,
        routes: routes,
        title: "Winterland",
        debugShowCheckedModeBanner: false,
        pageRouteBuilder: WinterlandPageRoute.builder,
      ),
    );
  }
}
