{ locals.pas — local variable manipulation }
program locals;
var x, y, z: integer;
begin
  x := 42;
  y := x + 8;
  z := y * 2;
  write(x);
  writeln;
  write(y);
  writeln;
  write(z);
  writeln;

  { swap using temp }
  z := x;
  x := y;
  y := z;
  write(x);
  writeln;
  write(y);
  writeln
end.
