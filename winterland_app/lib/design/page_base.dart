import 'package:flutter/widgets.dart';

import 'theme.dart';

class PageBase extends StatelessWidget {
  final Widget? child;

  const PageBase(this.child, {super.key});

  @override
  Widget build(BuildContext context) {
    final bg = Theme.of(context).pageBackgroundColor;
    return Container(color: bg, child: child, );
  }
}
