import 'package:flutter_adaptive_scaffold/flutter_adaptive_scaffold.dart';
import 'package:path_provider/path_provider.dart';
import 'package:rinf/rinf.dart';
import 'package:spectecle/ui/continue_reading.dart';
import 'package:spectecle/ui/library.dart';
import 'src/bindings/bindings.dart';
import 'package:flutter/material.dart';

Future<void> main() async {
  await initializeRust(assignRustSignal);
  AppSupportDirectory(
    path: (await getApplicationSupportDirectory()).path,
  ).sendSignalToRust();
  WidgetsFlutterBinding.ensureInitialized();
  PaintingBinding.instance.imageCache.maximumSizeBytes = 200_000_000; // 200M
  PaintingBinding.instance.imageCache.maximumSize =
      400; // max 400 images cached
  runApp(MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(title: 'Spectecle', home: Home());
  }
}

class Home extends StatefulWidget {
  Home({super.key});
  final _pages = [ContinueReading.getInstance(), Library.getInstance()];
  @override
  State<Home> createState() => _HomeState();
}

class _HomeState extends State<Home> {
  int _selectedIndex = 1;
  @override
  Widget build(BuildContext context) {
    return AdaptiveScaffold(
      transitionDuration: Duration.zero,
      destinations: [
        NavigationDestination(icon: Icon(Icons.menu_book), label: "Reading"),
        NavigationDestination(icon: Icon(Icons.shelves), label: "Library"),
      ],
      selectedIndex: _selectedIndex,
      onSelectedIndexChange: (index) {
        setState(() => _selectedIndex = index);
      },
      body: (context) {
        return IndexedStack(index: _selectedIndex, children: widget._pages);
      },
    );
  }
}
