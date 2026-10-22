import 'package:flutter/widgets.dart';
import 'package:gap/gap.dart';
import 'package:wide_color_tool/wide_color_tool.dart';
import 'package:winterland_app/utils/let.dart';
@immutable
class ThemeData {
  final Color primaryColor;
  final Color? _textSelectionColor;
  final Color pageBackgroundColor;
  final Color foregroundForPrimaryColor;
  final Color foregroundColor;
  final Color disabledColor;
  final Color buttonHoverColor;
  final Color buttonFocusColor;
  final Color buttonActiveColor;
  final Color inputBackgroundColor;
  final Color inputBorderColor;

  final TextStyle buttonTextStyle;
  final TextStyle inputTextStyle;

  final TextStyle displayTextStyle;
  final TextStyle headline1TextStyle;
  final TextStyle headline2TextStyle;
  final TextStyle headline3TextStyle;
  final TextStyle labelTextStyle;
  final TextStyle paragraphTextStyle;
  final TextStyle monospaceTextStyle;

  final Radius? _defaultBorderRadius;
  final double paddingUnit;

  Radius get defaultBorderRadius =>
      _defaultBorderRadius ?? const Radius.circular(4);

  Color get textSelectionColor =>
      _textSelectionColor ?? primaryColor.withAlpha((255 * 0.5).round());

  const ThemeData({
    this.primaryColor = const Color(0xFF1072FF),
    this.pageBackgroundColor = const Color(0xFF111111),
    this.foregroundForPrimaryColor = const Color(0xFFFFFFFF),
    this.foregroundColor = const Color(0xFFFFFFFF),
    this.disabledColor = const Color(0xFF666666),
    this.buttonHoverColor = const Color(0xFF4486EA),
    this.buttonFocusColor = const Color(0xFF3075DE),
    this.buttonActiveColor = const Color(0xFF094198),
    this.inputBackgroundColor = const Color(0xFF131313),
    this.inputBorderColor = const Color(0x20FFFFFF),
    this.paddingUnit = 4,
    this.buttonTextStyle = const TextStyle(
      fontWeight: FontWeight.w600,
    ),
    this.inputTextStyle = const TextStyle(
      fontSize: 14,
    ),
    this.displayTextStyle = const TextStyle(
      fontSize: 32,
      fontWeight: FontWeight.w600,
      height: 1.5,
    ),
    this.headline1TextStyle = const TextStyle(
      fontSize: 24,
      fontWeight: FontWeight.w600,
      height: 1.5,
    ),
    this.headline2TextStyle = const TextStyle(
      fontSize: 20,
      fontWeight: FontWeight.w500,
      height: 1.5,
    ),
    this.headline3TextStyle = const TextStyle(
      fontSize: 16,
      fontWeight: FontWeight.w500,
      height: 1.5,
    ),
    this.labelTextStyle = const TextStyle(
      fontSize: 12,
      fontWeight: FontWeight.w500,
      height: 1.5,
    ),
    this.paragraphTextStyle = const TextStyle(
      fontSize: 14,
      fontWeight: FontWeight.w400,
      height: 1.5,
    ),
    this.monospaceTextStyle = const TextStyle(
      fontFamily: 'JetBrains Mono',
      fontSize: 13,
      fontWeight: FontWeight.w400,
      height: 1.5,
    ),
    this._defaultBorderRadius,
    this._textSelectionColor,
  });
  Widget _text(
      String text,
      TextStyle style, [
        TextStyle? override,
      ]) {
    return Text(
      text,
      style: style.merge(override),
    );
  }

  Widget text(String text, [TextStyle? override]) =>
      _text(text, paragraphTextStyle, override);

  Widget textDisplay(String text, [TextStyle? override]) =>
      _text(text, displayTextStyle, override);

  Widget textHeadline1(String text, [TextStyle? override]) =>
      _text(text, headline1TextStyle, override);

  Widget textHeadline2(String text, [TextStyle? override]) =>
      _text(text, headline2TextStyle, override);

  Widget textHeadline3(String text, [TextStyle? override]) =>
      _text(text, headline3TextStyle, override);

  Widget textLabel(String text, [TextStyle? override]) =>
      _text(text, labelTextStyle, override);

  Widget textMonospace(String text, [TextStyle? override]) =>
      _text(text, monospaceTextStyle, override);

  Widget textButton(String text, [TextStyle? override]) =>
      _text(text, buttonTextStyle, override);

  Widget textInput(String text, [TextStyle? override]) =>
      _text(text, inputTextStyle, override);
}

class Theme extends InheritedWidget {
  final ThemeData data;

  const Theme({required this.data, required super.child, super.key});

  static ThemeData of(BuildContext context) {
    final theme = context.dependOnInheritedWidgetOfExactType<Theme>();
    assert(theme != null, 'No Winterland Theme found in the widget tree.');
    return theme!.data;
  }

  @override
  bool updateShouldNotify(Theme oldWidget) => data != oldWidget.data;
}
abstract class Spacing {
  static EdgeInsets symmetric(BuildContext context, {int x = 0, int y = 0}) => Theme.of(context).paddingUnit.let((paddingUnit) => EdgeInsets.symmetric(horizontal: paddingUnit * x, vertical: paddingUnit * y));
  static EdgeInsets weird(BuildContext context, {int left = 0, int right = 0, int top = 0, int bottom = 0}) => Theme.of(context).paddingUnit.let((paddingUnit) => EdgeInsets.only(left: paddingUnit * left, right: paddingUnit * right, top: paddingUnit * top, bottom: paddingUnit * bottom));
  static EdgeInsets all(BuildContext context, int value) => Theme.of(context).paddingUnit.let((paddingUnit) => EdgeInsets.all(paddingUnit*value));
  static double get(BuildContext context, int value) => Theme.of(context).paddingUnit * value;
  static Gap gap(BuildContext context, int value) => .new(get(context, value));
}
