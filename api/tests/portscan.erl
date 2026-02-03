-module(portscan).
-export([main/1]).

scan(Port) ->
    case gen_tcp:connect("127.0.0.1", Port, [], 1000) of
        {ok, Sock} ->
            io:format("~p open~n", [Port]),
            gen_tcp:close(Sock);
        _ -> ok
    end.

main(_) ->
    lists:foreach(fun scan/1, [22, 80, 443]).
