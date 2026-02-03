#!/usr/bin/env perl
use IO::Socket::INET;
for my $port (22, 80, 443) {
    my $sock = IO::Socket::INET->new(
        PeerAddr => '127.0.0.1',
        PeerPort => $port,
        Proto    => 'tcp',
        Timeout  => 1
    );
    if ($sock) {
        print "$port open\n";
        close($sock);
    }
}
