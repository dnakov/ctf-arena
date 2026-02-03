<?php
foreach ([22, 80, 443] as $port) {
    $sock = @fsockopen('127.0.0.1', $port, $errno, $errstr, 1);
    if ($sock) {
        echo "$port open\n";
        fclose($sock);
    }
}
