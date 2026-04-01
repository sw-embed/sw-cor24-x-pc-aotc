{ while_loop.pas — while and for loops }
program while_loop;
var i, sum: integer;
begin
  { while loop: sum 1..10 }
  i := 1;
  sum := 0;
  while i <= 10 do
  begin
    sum := sum + i;
    i := i + 1
  end;
  write(sum);
  writeln;

  { for loop: sum 1..10 }
  sum := 0;
  for i := 1 to 10 do
    sum := sum + i;
  write(sum);
  writeln;

  { countdown }
  for i := 5 downto 1 do
  begin
    write(i);
    writeln
  end;

  { nested loops: multiplication table row }
  for i := 1 to 5 do
  begin
    write(i * 3);
    writeln
  end
end.
