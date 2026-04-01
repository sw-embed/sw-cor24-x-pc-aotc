{ arithmetic.pas — basic integer arithmetic operations }
program arithmetic;
var a, b, c: integer;
begin
  a := 10;
  b := 3;

  { addition }
  c := a + b;
  write(c);
  writeln;

  { subtraction }
  c := a - b;
  write(c);
  writeln;

  { multiplication }
  c := a * b;
  write(c);
  writeln;

  { division }
  c := a div b;
  write(c);
  writeln;

  { modulo }
  c := a mod b;
  write(c);
  writeln;

  { negation }
  c := -a;
  write(c);
  writeln;

  { compound expression }
  c := (a + b) * (a - b);
  write(c);
  writeln
end.
