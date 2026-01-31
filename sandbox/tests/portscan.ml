let scan port =
  try
    let sock = Unix.socket Unix.PF_INET Unix.SOCK_STREAM 0 in
    Unix.connect sock (Unix.ADDR_INET (Unix.inet_addr_of_string "127.0.0.1", port));
    Printf.printf "%d open\n" port;
    Unix.close sock
  with _ -> ()

let () = List.iter scan [22; 80; 443]
