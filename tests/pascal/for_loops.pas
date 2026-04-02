{ for_loops.pas — various for loop patterns }
program for_loops;
var i, sum, fact: integer;
begin
  { simple for to }
  sum := 0;
  for i := 1 to 10 do
    sum := sum + i;
  write(sum);
  writeln;

  { for downto }
  sum := 0;
  for i := 10 downto 1 do
    sum := sum + i;
  write(sum);
  writeln;

  { factorial via for loop }
  fact := 1;
  for i := 1 to 6 do
    fact := fact * i;
  write(fact);
  writeln;

  { for loop with single iteration }
  sum := 0;
  for i := 5 to 5 do
    sum := sum + i;
  write(sum);
  writeln;

  { for loop counting characters }
  for i := 65 to 74 do
    write(i);
  writeln
end.
