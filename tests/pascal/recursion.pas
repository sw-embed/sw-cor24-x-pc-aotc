{ recursion.pas — iterative equivalents of recursive algorithms }
{ Note: p24p Phase 1 doesn't support user functions; uses iterative approach }
program recursion;
var i, n, fact, a, b, temp: integer;
begin
  { iterative factorial 0..7 }
  for n := 0 to 7 do
  begin
    fact := 1;
    for i := 2 to n do
      fact := fact * i;
    write(fact);
    writeln
  end;

  { iterative fibonacci 0..10 }
  a := 0;
  b := 1;
  write(a);
  writeln;
  write(b);
  writeln;
  for i := 2 to 10 do
  begin
    temp := a + b;
    a := b;
    b := temp;
    write(b);
    writeln
  end
end.
