import 'package:flutter/material.dart';

class ContinueReading extends StatefulWidget {
  static ContinueReading? _instance;
  const ContinueReading({super.key});

  static ContinueReading getInstance() {
    _instance ??= ContinueReading();
    return _instance!;
  }

  @override
  State<ContinueReading> createState() => _ContinueReadingState();
}

class _ContinueReadingState extends State<ContinueReading> {
  @override
  Widget build(BuildContext context) {
    return const Placeholder();
  }
}
