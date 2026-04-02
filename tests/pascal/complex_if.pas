{ complex_if.pas — complex conditional patterns }
program complex_if;
var x, y, result: integer;
begin
  { deeply nested if-else }
  x := 7;
  y := 3;
  if x > 5 then
    if y > 5 then
      result := 1
    else if y > 2 then
      result := 2
    else
      result := 3
  else
    result := 4;
  write(result);
  writeln;

  { multiple conditions with arithmetic }
  if x + y > 15 then
    write(1)
  else if x + y > 10 then
    write(2)
  else if x + y > 5 then
    write(3)
  else
    write(4);
  writeln;

  { if-else with different comparison operators }
  if x = 7 then write(1) else write(0);
  if x <> 7 then write(1) else write(0);
  if x < 10 then write(1) else write(0);
  if x <= 7 then write(1) else write(0);
  if x > 5 then write(1) else write(0);
  if x >= 7 then write(1) else write(0);
  writeln;

  { boolean-like expressions in conditions }
  x := 0;
  if x = 0 then
    write(1)
  else
    write(0);
  writeln;

  x := 1;
  if x = 0 then
    write(0)
  else
    write(1);
  writeln
end.
