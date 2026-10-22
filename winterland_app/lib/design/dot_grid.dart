
import 'package:flutter/widgets.dart';
import 'package:winterland_app/design/theme.dart';

class DotGrid extends StatelessWidget {
  const DotGrid(this.child, {
    super.key,
    this.spacing = 6,
    this.radius = 1,
    this.color = const Color(0xFF454545),
  });

  final Widget child;
  final int spacing;
  final double radius;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      painter: _DotGridPainter(
        spacing: Spacing.get(context, spacing),
        radius: radius,
        color: color,
      ),
      child: child,
    );
  }
}

class _DotGridPainter extends CustomPainter {
  const _DotGridPainter({
    required this.spacing,
    required this.radius,
    required this.color,
  });

  final double spacing;
  final double radius;
  final Color color;


  @override
  void paint(Canvas canvas, Size size) {
    if (spacing <= 0 || radius <= 0) return;

    canvas.save();
    canvas.translate(size.width / 2, size.height / 2);

    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.fill;

    final halfWidth = size.width / 2;
    final halfHeight = size.height / 2;

    final columns = (halfWidth / spacing).ceil();
    final rows = (halfHeight / spacing).ceil();

    for (int row = -rows; row <= rows; row++) {
      for (int col = -columns; col <= columns; col++) {
        canvas.drawCircle(
          Offset(col * spacing, row * spacing),
          radius,
          paint,
        );
      }
    }

    canvas.restore();
  }


  @override
  bool shouldRepaint(covariant _DotGridPainter oldDelegate) {
    return spacing != oldDelegate.spacing ||
        radius != oldDelegate.radius ||
        color != oldDelegate.color;
  }
}
