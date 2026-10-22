import 'package:flutter/widgets.dart';
import 'package:wide_color_tool/wide_color_tool.dart';
import 'package:winterland_app/design/focus_outline.dart';

import 'button.dart';
import 'theme.dart';

class TextInput extends StatefulWidget {
  final TextEditingController? controller;
  final FocusNode? focusNode;
  final String? hintText;
  final ValueChanged<String>? onChanged;
  final Widget? leadingButtonIcon;

  final String? leadingButtonSemanticText;

  final VoidCallback? onLeadingButtonPressed;

  const TextInput({
    this.controller,
    this.focusNode,
    this.hintText,
    this.onChanged,
    this.leadingButtonIcon,
    this.onLeadingButtonPressed,
    this.leadingButtonSemanticText,
    super.key,
  });

  @override
  State<TextInput> createState() => _TextInputState();
}

class _TextInputState extends State<TextInput>
    implements TextSelectionGestureDetectorBuilderDelegate {
  late TextEditingController _controller;
  late FocusNode _focusNode;
  FocusNode? _leadingButtonFocusNode;
  late bool _ownsController;
  late bool _ownsFocusNode;

  bool hover = false;

  @override
  void initState() {
    super.initState();
    _updateControllers();
  }

  @override
  void dispose() {
    super.dispose();
    if (_ownsController) _controller.dispose();
    if (_ownsFocusNode) _focusNode.dispose();
    _leadingButtonFocusNode?.dispose();
  }

  @override
  void didUpdateWidget(TextInput oldInput) {
    super.didUpdateWidget(oldInput);
    _updateControllers(oldInput);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return MouseRegion(
      cursor: SystemMouseCursors.text,
      onHover: (_) {
        if (!hover) setState(() => hover = true);
      },
      onExit: (_) {
        if (hover) setState(() => hover = false);
      },
      child: ListenableBuilder(
        listenable: _focusNode,
        builder: (context, child) {
          Color borderColor;

          if (_focusNode.hasFocus) {
            borderColor = theme.primaryColor;
          } else if (!hover) {
            borderColor = theme.inputBorderColor;
          } else {
            borderColor = WideColor.fromColor(theme.primaryColor)
                .mix(
                  WideColor.fromColor(theme.inputBorderColor),
                  source: ColorSource.rgb,
                )
                .color;
          }

          return FocusOutline(
            show: _focusNode.hasFocus,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: theme.inputBackgroundColor,
                border: Border.all(color: borderColor),
                borderRadius: BorderRadius.all(theme.defaultBorderRadius),
              ),
              child: child,
            ),
          );
        },
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisAlignment: MainAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              if (widget.leadingButtonIcon != null) ...[
                ButtonSkeleton(
                  focusNode: _leadingButtonFocusNode,
                  semanticLabel: widget.leadingButtonSemanticText,
                  onPressed: widget.onLeadingButtonPressed,
                  meat: ButtonMeat(
                    padding: Spacing.symmetric(context, x: 3, y: 2),
                    backgroundColor: ButtonStateProperty((state) {
                      if (state == ButtonState.pressed) {
                        return theme.foregroundColor.withValues(alpha: 0.15);
                      }
          
                      if (state == ButtonState.hovered ||
                          state == ButtonState.focused) {
                        return theme.foregroundColor.withValues(alpha: 0.10);
                      }
          
                      return theme.foregroundColor.withValues(alpha: 0.0);
                    }),

                    foregroundColor: ButtonStateProperty((state) {
                      if (state == ButtonState.disabled) {
                        return theme.disabledColor;
                      }
                      if (state == ButtonState.hovered ||
                          state == ButtonState.focused ||
                          state == ButtonState.pressed) {
                        return theme.primaryColor;
                      }
                      return theme.foregroundColor;
                    }),
                    borderRadius: BorderRadius.only(topLeft: theme.defaultBorderRadius, bottomLeft: theme.defaultBorderRadius),
                  ),
                  child: widget.leadingButtonIcon!,
                ),
              ],
              Expanded(
                child: _selectionGestureDetectorBuilder.buildGestureDetector(
                  behavior: HitTestBehavior.translucent,
                  child: Padding(
                    padding: Spacing.symmetric(context, x: 4, y: 3).copyWith(left: widget.leadingButtonIcon != null ? 0 : null),
                    child: EditableText(
                      key: editableTextKey,
                      controller: _controller,
                      focusNode: _focusNode,
                      style: theme.inputTextStyle,
                      cursorColor: theme.primaryColor,
                      backgroundCursorColor: theme.inputBorderColor,
                      selectionColor: theme.textSelectionColor,
                      enableInteractiveSelection: true,
                      onChanged: widget.onChanged,
                      enableSuggestions: false,
                      enableIMEPersonalizedLearning: false,
                      stylusHandwritingEnabled: false,
                      rendererIgnoresPointer: true,
                      mouseCursor: MouseCursor.defer,
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _updateControllers([TextInput? oldInput]) {
    if (oldInput == null || oldInput.controller != widget.controller) {
      if (oldInput != null && _ownsController) {
        _controller.dispose();
      }

      _controller = widget.controller ?? TextEditingController();
      _ownsController = widget.controller == null;
    }

    if (oldInput == null || oldInput.focusNode != widget.focusNode) {
      if (oldInput != null && _ownsFocusNode) {
        _focusNode.dispose();
        _leadingButtonFocusNode?.dispose();
      }

      _ownsFocusNode = widget.focusNode == null;
      _focusNode = widget.focusNode ?? FocusNode();
      if(widget.leadingButtonIcon != null) {
        _leadingButtonFocusNode = FocusNode();
      }
    }
  }

  late final TextSelectionGestureDetectorBuilder _selectionGestureDetectorBuilder =
      .new(delegate: this);

  @override
  final GlobalKey<EditableTextState> editableTextKey =
      GlobalKey<EditableTextState>();

  @override
  bool get forcePressEnabled => false;

  @override
  bool get selectionEnabled => true;
}
