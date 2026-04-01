{ nested_calls.pas — complex expression nesting and sequential operations }
program nested_calls;
var a, b, c, r: integer;
begin
  a := 10;
  b := 20;
  c := 30;

  { deeply nested arithmetic }
  r := ((a + b) * (c - a)) div (b - a);
  write(r);
  writeln;

  { chained assignments }
  a := 5;
  b := a * 3;
  c := b + a;
  r := c * 2;
  write(r);
  writeln;

  { mixed operations }
  write(a + b + c);
  writeln;
  write(a * b - c);
  writeln;
  write((a + 1) * (b - 1));
  writeln;

  { sequential writes testing stack cleanup }
  write(1);
  write(2);
  write(3);
  write(4);
  write(5);
  writeln;

  { expression with all operators }
  r := 100 - (a * b) + (c div a) + (c mod b);
  write(r);
  writeln
end.
