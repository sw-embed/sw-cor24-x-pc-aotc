{ if_else.pas — conditional branching }
program if_else;
var x: integer;
begin
  x := 10;

  if x > 5 then
    write(1)
  else
    write(0);
  writeln;

  if x < 5 then
    write(1)
  else
    write(0);
  writeln;

  if x = 10 then
    write(1)
  else
    write(0);
  writeln;

  { nested if }
  if x > 0 then
    if x < 20 then
      write(1)
    else
      write(0)
  else
    write(0);
  writeln;

  { chained conditions }
  if x > 100 then
    write(3)
  else if x > 50 then
    write(2)
  else if x > 0 then
    write(1)
  else
    write(0);
  writeln
end.
