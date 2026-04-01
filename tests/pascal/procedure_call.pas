{ procedure_call.pas — runtime procedure calls (write, writeln) }
{ Note: p24p Phase 1 doesn't support user procedures; tests runtime calls }
program procedure_call;
var x, y: integer;
begin
  x := 42;
  y := 99;

  { write integer }
  write(x);
  writeln;

  { write literal }
  write(100);
  writeln;

  { write expression }
  write(x + y);
  writeln;

  { multiple writes }
  write(1);
  write(2);
  write(3);
  writeln;

  { write negative }
  write(-7);
  writeln
end.
