import Network.Socket
import Control.Exception (try, SomeException)

scan :: Int -> IO ()
scan port = do
    let hints = defaultHints { addrSocketType = Stream }
    addr:_ <- getAddrInfo (Just hints) (Just "127.0.0.1") (Just $ show port)
    sock <- socket (addrFamily addr) (addrSocketType addr) (addrProtocol addr)
    result <- try (connect sock (addrAddress addr)) :: IO (Either SomeException ())
    case result of
        Right _ -> putStrLn $ show port ++ " open"
        Left _ -> return ()
    close sock

main :: IO ()
main = mapM_ scan [22, 80, 443]
