
import 'package:flutter/widgets.dart';
import 'package:winterland_app/design/theme.dart';

class Card extends StatelessWidget {
  const Card({
    super.key,
    required this.child,
    this.padding = 3,
    this.backgroundColor,
    this.borderColor,
    this.borderRadius,
  });

  final Widget child;
  final int padding;
  final Color? backgroundColor;
  final Color? borderColor;
  final BorderRadius? borderRadius;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      padding: Spacing.all(context, padding),
      decoration: BoxDecoration(
        color: backgroundColor ?? theme.inputBackgroundColor,
        border: Border.all(
          color: borderColor ?? theme.inputBorderColor,
        ),
        borderRadius: borderRadius ??
            BorderRadius.all(theme.defaultBorderRadius),
      ),
      child: child,
    );
  }
}
