import 'dart:math';

import 'package:file_selector/file_selector.dart' show getDirectoryPath;
import 'package:flutter/widgets.dart';
import 'package:gap/gap.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:winterland_app/design/design.dart';

class OpenFlakePage extends StatefulWidget {
  final String defaultFlakePath;

  const OpenFlakePage({super.key, required this.defaultFlakePath});

  @override
  State<OpenFlakePage> createState() => _OpenFlakePageState();
}

class _OpenFlakePageState extends State<OpenFlakePage> {
  late final TextEditingController _pathTextController;

  @override
  initState() {
    super.initState();
    _pathTextController = .new(text: widget.defaultFlakePath);
  }

  @override
  build(context) {
    final theme = Theme.of(context);
    return PageBase(
      DotGrid(
          Center(
            child: Container(
              constraints: BoxConstraints.loose(Size(700, .infinity)),
              padding: Spacing.all(context, 5),
              child: Card(
                padding: 5,
                child: Column(
                  mainAxisSize: .min,
                  mainAxisAlignment: .center,
                  crossAxisAlignment: .start,
                  children: [
                    theme.textHeadline1("Welcome to Winterland ❄️"),
                    Spacing.gap(context, 1),
                    theme.text("To start, select the path to the directory of your flake:", TextStyle(
                      color: theme.foregroundColor.withAlpha(127)
                    )),
                    Spacing.gap(context, 5),
                    Row(
                      mainAxisAlignment: .center,
                      crossAxisAlignment: .center,
                      spacing: pow(2, 4).toDouble(),
                      children: [
                        Expanded(
                          flex: 1,
                          child: TextInput(
                            leadingButtonSemanticText: "Open Folder...",
                            leadingButtonIcon: Icon(LucideIcons.folderInput, size: 4*4,),
                            onLeadingButtonPressed: openFilePicker,
                            hintText: "Path",
                            controller: _pathTextController,
                          ),
                        ),
                        Button(label: "Open Flake", onPressed: () {})
                      ],
                    ),
                  ],
                ),
              ),
            ),
          )
      ),
    );
  }
  Future<void> openFilePicker() async {
    final directory = await getDirectoryPath();

    if (!mounted || directory == null) return;

    _pathTextController.text = directory;
  }
}
