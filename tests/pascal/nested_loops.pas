{ nested_loops.pas — nested while and for loops }
program nested_loops;
var i, j, sum: integer;
begin
  { nested for loops: sum of products }
  sum := 0;
  for i := 1 to 3 do
    for j := 1 to 4 do
      sum := sum + i * j;
  write(sum);
  writeln;

  { nested while loops: multiplication table }
  i := 1;
  while i <= 3 do
  begin
    j := 1;
    while j <= 3 do
    begin
      write(i * j);
      if j < 3 then
        write(32);
      j := j + 1
    end;
    writeln;
    i := i + 1
  end;

  { while inside for }
  for i := 1 to 3 do
  begin
    sum := 0;
    j := i;
    while j > 0 do
    begin
      sum := sum + j;
      j := j - 1
    end;
    write(sum);
    writeln
  end
end.
