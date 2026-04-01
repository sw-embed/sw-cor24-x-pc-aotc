{ globals.pas — global variable access patterns }
program globals;
var a, b, c, d, e: integer;
begin
  { basic assignment }
  a := 1;
  b := 2;
  c := 3;
  d := 4;
  e := 5;

  { read back in order }
  write(a);
  writeln;
  write(b);
  writeln;
  write(c);
  writeln;
  write(d);
  writeln;
  write(e);
  writeln;

  { cross-variable operations }
  a := b + c;
  b := d * e;
  c := b - a;
  write(a);
  writeln;
  write(b);
  writeln;
  write(c);
  writeln;

  { overwrite and reuse }
  d := a + b + c;
  e := d div 3;
  write(d);
  writeln;
  write(e);
  writeln
end.
