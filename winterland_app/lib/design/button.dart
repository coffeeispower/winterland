import 'package:flutter/widgets.dart';

import 'focus_outline.dart';
import 'theme.dart';

enum ButtonState {
  normal,
  hovered,
  focused,
  pressed,
  disabled,
}

@immutable
class ButtonStateProperty<T> {
  final T Function(ButtonState state) resolve;

  const ButtonStateProperty(this.resolve);

  factory ButtonStateProperty.all(T value) {
    return ButtonStateProperty((_) => value);
  }
}

@immutable
class ButtonMeat {
  final ButtonStateProperty<Color> backgroundColor;
  final ButtonStateProperty<Color> foregroundColor;
  final BorderRadius borderRadius;
  final Border? border;
  final EdgeInsetsGeometry padding;
  final Duration animationDuration;

  const ButtonMeat({
    required this.backgroundColor,
    required this.foregroundColor,
    this.borderRadius = BorderRadius.zero,
    this.border,
    this.padding = EdgeInsets.zero,
    this.animationDuration = const Duration(milliseconds: 100),
  });
}

class ButtonSkeleton extends StatefulWidget {
  final Widget child;
  final String? semanticLabel;
  final VoidCallback? onPressed;
  final FocusNode? focusNode;
  final ButtonMeat meat;

  const ButtonSkeleton({
    required this.child,
    required this.meat,
    this.semanticLabel,
    required this.onPressed,
    this.focusNode,
    super.key,
  });

  @override
  State<ButtonSkeleton> createState() => _ButtonSkeletonState();
}

class _ButtonSkeletonState extends State<ButtonSkeleton> {
  late FocusNode _focusNode;
  bool _ownsFocusNode = false;

  bool _isFocused = false;
  bool _isHovered = false;
  bool _isPressed = false;

  @override
  void initState() {
    super.initState();
    _updateFocusNode();
  }

  @override
  void didUpdateWidget(ButtonSkeleton oldWidget) {
    super.didUpdateWidget(oldWidget);

    if (oldWidget.focusNode != widget.focusNode) {
      if (_ownsFocusNode) {
        _focusNode.dispose();
      }
      _updateFocusNode();
    }
  }

  void _updateFocusNode() {
    _ownsFocusNode = widget.focusNode == null;
    _focusNode = widget.focusNode ?? FocusNode();
  }

  @override
  void dispose() {
    if (_ownsFocusNode) {
      _focusNode.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final enabled = widget.onPressed != null;
    late final ButtonState state;

    if (!enabled) {
      state = ButtonState.disabled;
    } else if (_isPressed) {
      state = ButtonState.pressed;
    } else if (_isFocused) {
      state = ButtonState.focused;
    } else if (_isHovered) {
      state = ButtonState.hovered;
    } else {
      state = ButtonState.normal;
    }

    final meat = widget.meat;

    return FocusableActionDetector(
      enabled: enabled,
      focusNode: _focusNode,
      onShowFocusHighlight: (value) {
        if (_isFocused != value) {
          setState(() => _isFocused = value);
        }
      },
      onShowHoverHighlight: (value) {
        if (_isHovered != value) {
          setState(() => _isHovered = value);
        }
      },
      child: MouseRegion(
        cursor: state != .disabled ? SystemMouseCursors.click : SystemMouseCursors.basic,
        child: Semantics(
          button: true,
          enabled: enabled,
          label: widget.semanticLabel,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: enabled ? widget.onPressed : null,
            onTapDown: enabled
                ? (_) {
              _focusNode.requestFocus();
              setState(() => _isPressed = true);
            }
                : null,
            onTapUp: enabled
                ? (_) => setState(() => _isPressed = false)
                : null,
            onTapCancel: enabled
                ? () => setState(() => _isPressed = false)
                : null,
            child: FocusOutline(
              show: _isFocused && !_isPressed,
              borderRadius: meat.borderRadius,
              child: AnimatedContainer(
                alignment: .center,
                duration: meat.animationDuration,
                decoration: BoxDecoration(
                  color: meat.backgroundColor.resolve(state),
                  borderRadius: meat.borderRadius,
                  border: meat.border,
                ),
                padding: meat.padding,
                child: IconTheme(
                  data: IconTheme.of(context).copyWith(
                    color: meat.foregroundColor.resolve(state),
                  ),
                  child: DefaultTextStyle(
                    style: DefaultTextStyle.of(context).style.copyWith(
                      color: meat.foregroundColor.resolve(state),
                    ),
                    child: IconTheme(
                      data: IconTheme.of(context).copyWith(
                        color: meat.foregroundColor.resolve(state),
                      ),
                      child: widget.child,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
class Button extends StatelessWidget {
  final String label;
  final VoidCallback? onPressed;
  final FocusNode? focusNode;

  const Button({
    required this.label,
    required this.onPressed,
    this.focusNode,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final enabled = onPressed != null;

    return ButtonSkeleton(
      semanticLabel: label,
      onPressed: onPressed,
      focusNode: focusNode,
      meat: ButtonMeat(
        backgroundColor: ButtonStateProperty((state) {
          return switch (state) {
            ButtonState.disabled => theme.disabledColor,
            ButtonState.pressed => theme.buttonActiveColor,
            ButtonState.focused => theme.buttonFocusColor,
            ButtonState.hovered => theme.buttonHoverColor,
            ButtonState.normal => theme.primaryColor,
          };
        }),
        foregroundColor: ButtonStateProperty.all(
          enabled
              ? theme.foregroundForPrimaryColor
              : theme.foregroundColor,
        ),
        borderRadius: BorderRadius.all(theme.defaultBorderRadius),
        padding: Spacing.symmetric(context, x: 4, y: 3),
      ),
      child: Text(
        label,
        style: theme.buttonTextStyle,
      ),
    );
  }
}