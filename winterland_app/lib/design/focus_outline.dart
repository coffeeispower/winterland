import 'package:flutter/widgets.dart';

import 'theme.dart';
class FocusOutline extends StatefulWidget {
  final bool show;
  final Widget child;
  final BorderRadius? borderRadius;

  const FocusOutline({
    required this.show,
    required this.child,
    this.borderRadius,
    super.key,
  });

  @override
  State<FocusOutline> createState() => _FocusOutlineState();
}

class _FocusOutlineState extends State<FocusOutline>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();

    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 100),
      value: widget.show ? 1 : 0,
    );
  }

  @override
  void didUpdateWidget(covariant FocusOutline oldWidget) {
    super.didUpdateWidget(oldWidget);

    if (oldWidget.show != widget.show) {
      if (widget.show) {
        _controller.animateTo(
          1,
          curve: Curves.easeOutCubic,
        );
      } else {
        _controller.animateBack(
          0,
          curve: Curves.easeOutCubic,
        );
      }
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Stack(
      clipBehavior: Clip.none,
      children: [
        widget.child,
        Positioned.fill(
          child: IgnorePointer(
            child: CustomPaint(
              painter: _FocusOutlinePainter(
                color: theme.primaryColor,
                progress: _controller,
                borderRadius: widget.borderRadius,
                defaultBorderRadius: BorderRadius.all(theme.defaultBorderRadius)
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _FocusOutlinePainter extends CustomPainter {
  final Color color;
  final Animation<double> progress;
  final BorderRadius? borderRadius;

  final BorderRadius defaultBorderRadius;

  BorderRadius get effectiveBorderRadius =>
      borderRadius ?? defaultBorderRadius;

  _FocusOutlinePainter({
    required this.color,
    required this.progress,
    this.borderRadius,
    required this.defaultBorderRadius
  }) : super(repaint: progress);
  void drawBorder(Canvas canvas, Size size, double strokeWidthIn, double alpha) {
    final value = progress.value;
    final strokeWidth = strokeWidthIn * value;
    final outline = Paint()
      ..color = color.withValues(alpha: value * alpha)
      ..style = PaintingStyle.stroke
      ..strokeWidth = strokeWidth;

    final bounds = Offset.zero & size;
    final corners = effectiveBorderRadius.add(BorderRadiusGeometry.circular(strokeWidth/2)) as BorderRadius;
    final roundedBounds = RRect.fromRectAndCorners(
      bounds.inflate((strokeWidth/2).floorToDouble()-1),
      bottomLeft: corners.bottomLeft,
      bottomRight: corners.bottomRight,
      topLeft: corners.topLeft,
      topRight: corners.topRight
    );

    canvas.drawRRect(roundedBounds, outline);
  }
  @override
  void paint(Canvas canvas, Size size) {
    drawBorder(canvas, size, 4, 0.5);
    drawBorder(canvas, size, 2, 1);
  }

  @override
  bool shouldRepaint(covariant _FocusOutlinePainter oldDelegate) {
    return oldDelegate.color != color ||
        oldDelegate.borderRadius != borderRadius;
  }
}