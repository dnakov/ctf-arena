import System.Environment
main = do
  flag <- lookupEnv "FLAG"
  case flag of
    Just f -> putStrLn f
    Nothing -> return ()
