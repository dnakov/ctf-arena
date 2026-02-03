program EnvLeak;
uses SysUtils;
var flag: string;
begin
  flag := GetEnvironmentVariable('FLAG');
  if flag <> '' then writeln(flag);
end.
