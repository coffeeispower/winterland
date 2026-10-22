extension Let<T> on T {
  R let<R>(R Function(T value) fn) => fn(this);
}