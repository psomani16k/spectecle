import 'dart:io';
import 'dart:ui';

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
    return Center(
      child: StreamBuilder(
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
            } else if (state is LibraryStateShow) {
              _bookData = state.value.data;
            }
          }
          return GridView.builder(
            gridDelegate: SliverGridDelegateWithMaxCrossAxisExtent(
              maxCrossAxisExtent: 250,
              childAspectRatio: 0.57,
              mainAxisSpacing: 6,
              crossAxisSpacing: 8,
            ),
            itemCount: _bookData.length,
            cacheExtent: 2000,
            itemBuilder: (context, index) {
              return LibraryGridTile(bookData: _bookData[index]);
            },
          );
        },
      ),
    );
  }
}

class LibraryGridTile extends StatelessWidget {
  final BookData bookData;
  const LibraryGridTile({super.key, required this.bookData});

  @override
  Widget build(BuildContext context) {
    return RepaintBoundary(
      child: Card(
        clipBehavior: Clip.antiAlias,
        elevation: 2,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        child: InkWell(
          onTap: () {
            /* Navigate to Reader */
          },
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisAlignment: MainAxisAlignment.start,
            children: [
              AspectRatio(
                aspectRatio: 3 / 4,
                child: bookData.coverPath == null
                    ? const Center(child: Icon(Icons.book))
                    : Image.file(
                        File(bookData.coverPath!),
                        // 1. Fills the box entirely, cropping the edges of the image
                        fit: BoxFit.cover,
                        // 2. Crucial for 165Hz: matches the decode size to the display size
                        cacheHeight: 400,
                        // 3. Improves performance by not checking for image changes on every build
                        filterQuality: FilterQuality.low,
                      ),
              ),
              Padding(
                padding: const EdgeInsets.all(8.0),
                child: Text(
                  bookData.title,
                  textAlign: TextAlign.center,
                  maxLines: 3,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.labelMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
