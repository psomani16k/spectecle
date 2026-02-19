import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:spectecle/src/bindings/signals/signals.dart';

class Library extends StatefulWidget {
  static Library? _instance;
  const Library({super.key});

  static Library getInstance() {
    _instance ??= Library();
    return _instance!;
  }

  @override
  State<Library> createState() => _LibraryState();
}

class _LibraryState extends State<Library> {
  List<BookData> _bookData = [];

  @override
  Widget build(BuildContext context) {
    return StreamBuilder(
      stream: LibraryState.rustSignalStream,
      builder: (context, snapshot) {
        if (snapshot.hasData) {
          final state = snapshot.data!.message;
          if (state is LibraryStateShow) {
            _bookData = state.value.data;
          } else if (state is LibraryStateRefreshingCache) {
            return Center(
              child: Column(
                children: [
                  CircularProgressIndicator(),
                  Text("Refreshing Cache, just a second!"),
                ],
              ),
            );
          } else if (state is LibraryStateRebuildingCache) {
            return Center(
              child: Column(
                children: [
                  CircularProgressIndicator(),
                  Text("Building Cache, this could take a minute!"),
                ],
              ),
            );
          } else if (state is LibraryStateNoLibraryAvailable) {
            return Center(
              child: Column(
                children: [
                  Text("No Library selected"),
                  FilledButton.tonal(
                    onPressed: () async {
                      final newLib = await FilePicker.platform
                          .getDirectoryPath();
                      if (newLib != null) {
                        AddToLibrary(path: newLib).sendSignalToRust();
                      }
                    },
                    child: Text("Add Library"),
                  ),
                ],
              ),
            );
          }
        }
        return GridView.builder(
          gridDelegate: SliverGridDelegateWithMaxCrossAxisExtent(
            maxCrossAxisExtent: MediaQuery.widthOf(context),
          ),
          itemCount: _bookData.length,
          itemBuilder: (context, index) {
            final coverPath = _bookData[index].coverPath;
            final title = _bookData[index].title;
            return GridTile(
              footer: Text(title),
              child: coverPath != null
                  ? Image.file(File(coverPath))
                  : Icon(Icons.book_outlined),
            );
          },
        );
      },
    );
  }
}
