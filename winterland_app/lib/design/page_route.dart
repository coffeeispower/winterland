import 'dart:ui' as ui;

import 'package:flutter/widgets.dart';

abstract final class WinterlandPageRoute {
  WinterlandPageRoute._();

  static PageRoute<T> builder<T>(
    RouteSettings settings,
    WidgetBuilder builder,
  ) {
    return PageRouteBuilder<T>(
      settings: settings,
      opaque: false,
      transitionDuration: const Duration(milliseconds: 500),
      reverseTransitionDuration: const Duration(milliseconds: 350),
      pageBuilder: (context, animation, secondaryAnimation) {
        return builder(context);
      },
      transitionsBuilder: (context, animation, secondaryAnimation, child) {
        // The new page grows into focus as it comes forward.
        final newPage = CurvedAnimation(
          parent: animation,
          curve: Curves.easeOutCubic,
          reverseCurve: Curves.easeInCubic,
        );

        // The page below recedes into the distance.
        final pageBelow = CurvedAnimation(
          parent: secondaryAnimation,
          curve: Curves.easeOutCubic,
          reverseCurve: Curves.easeInCubic,
        );

        final newPageScale = Tween<double>(
          begin: 1.5,
          end: 1.0,
        ).animate(newPage);

        final newPageBlurProgress = CurvedAnimation(
          parent: animation,
          curve: Curves.easeOutQuart,
          reverseCurve: Curves.easeInQuart,
        );
        final newPageBlur = Tween<double>(
          begin: 20.0,
          end: 0.0,
        ).animate(newPageBlurProgress);

        final pageBelowScale = Tween<double>(
          begin: 1.0,
          end: 0.8,
        ).animate(pageBelow);

        final pageBelowOpacity = Tween<double>(
          begin: 1.0,
          end: 0.0,
        ).animate(pageBelow);

        return AnimatedBuilder(
          animation: Listenable.merge([
            newPage,
            newPageBlurProgress,
            pageBelow,
          ]),
          child: child,
          builder: (context, child) {
            return Opacity(
              opacity: newPage.value,
              child: Transform.scale(
                scale: newPageScale.value,
                alignment: Alignment.center,
                child: ImageFiltered(
                  imageFilter: ui.ImageFilter.blur(
                    sigmaX: newPageBlur.value,
                    sigmaY: newPageBlur.value,
                  ),
                  child: Transform.scale(
                    scale: pageBelowScale.value,
                    alignment: Alignment.center,
                    child: Opacity(
                      opacity: pageBelowOpacity.value,
                      child: child,
                    ),
                  ),
                ),
              ),
            );
          },
        );
      },
    );
  }
}
