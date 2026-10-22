import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:winterland_app/app.dart';
import 'package:winterland_app/pages/flake_workspace_page.dart';
import 'package:winterland_app/pages/open_flake_page.dart';
import 'package:winterland_app/design/page_route.dart';

void main() {
  test('builds the configured Winterland page route', () {
    final route = WinterlandPageRoute.builder<void>(
      const RouteSettings(name: '/details'),
      (_) => const SizedBox(),
    );

    expect(route.settings.name, '/details');
    expect(route.opaque, isFalse);
    expect(route.transitionDuration, const Duration(milliseconds: 500));
    expect(route.reverseTransitionDuration, const Duration(milliseconds: 350));
  });

  testWidgets('registers the named page routes', (tester) async {
    await tester.pumpWidget(WinterlandApp('test-flake'));

    expect(find.byType(OpenFlakePage), findsOneWidget);

    final navigator = tester.state<NavigatorState>(find.byType(Navigator));
    navigator.pushNamed('/workspace');
    await tester.pumpAndSettle();
    expect(find.byType(FlakeWorkspacePage), findsOneWidget);

    navigator.pop();
    await tester.pumpAndSettle();
    expect(find.byType(OpenFlakePage), findsOneWidget);
  });
}
