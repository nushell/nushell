def f [
  a: list<int>
  b: list<>
  c: list< string >
  d: record<>
  e: record<a: int b: string>
  f: record<a: int, b: string>
  g: record<a b: int>
  h: table<a: int>
  i: table<>
  j: oneof<int, string, nothing>
  k: oneof<>
  l: list<list<list<int>>>
  m: record<a: list<record<b: int>>>
  n: record<"quoted key": int, 'other': string>
] { }
