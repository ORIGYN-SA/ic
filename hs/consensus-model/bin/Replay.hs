{-# LANGUAGE ImplicitParams #-}
{-# LANGUAGE LambdaCase #-}
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ViewPatterns #-}

module Main where

import Bisimilar
import Consensus
import Control.DeepSeq
import Control.Monad
import Control.Monad.Trans.Class
import Control.Monad.Trans.State
import Data.Foldable
import Data.List (permutations, subsequences)
import qualified Data.Map as M
import JSON
import Monad
import System.Environment
import System.IO (IOMode (ReadMode), stdin, withFile)
import Test.Tasty.HUnit hiding (assert)
import Text.Show.Pretty
import Types
import Utils

replay :: [TraceEvent] -> Assertion
replay (FoundSubnets _subnetIds : xs) = do
  (network', incoming') <-
    (\f -> foldrM f (M.empty, []) xs) $ \x (net, rest) -> case x of
      FoundSubnet subnetId nodeIds -> do
        replicaIds <- register_nodes nodeIds
        pure (M.insert subnetId replicaIds net, rest)
      _ -> do
        pure (net, x : rest)
  let (topo, cfg0) = create_topology network'
  let ?topo = topo
  runConsensus cfg0 0 $
    (\f -> foldM_ f [] incoming') $ \acc -> \case
      FoundSubnets _ -> error "handled above"
      FoundSubnet _ _ -> error "handled above"
      ArtifactSeen x -> acc <$ post x
      ChangeActionSeen x -> case x of
        AddToValidated (RandomTapeShareMsg _) -> pure acc
        AddToValidated (RandomTapeMsg _) -> pure acc
        AddToValidated (CatchUpPackageShareMsg _) -> pure acc
        AddToValidated (CatchUpPackageMsg _) -> pure acc
        AddToValidated (RandomBeaconMsg rb) -> do
          modifyRandomBeacons $ \rb' ->
            if rb_height rb' == rb_height rb
              then rb' {rb_parent = rb_parent rb}
              else rb'
          pure (x : acc)
        AddToValidated (BlockProposalMsg bp) -> do
          modifyBlocks $ \b' ->
            if b_height b' == b_height (bp_block bp)
              then b' {b_parent = b_parent (bp_block bp)}
              else b'
          pure (x : acc)
        PurgeUnvalidatedBelow _ -> pure acc
        _ -> pure (x : acc)
      ApplyChanges time -> do
        -- (2020-11-24): When we see "apply_changes" in the real
        -- consensus log, we look to see whether the set of changes from
        -- that "step" occur within the union-based set generated by the
        -- reference implementation. If so, we apply those changes only and
        -- progress.

        -- (2020-11-24): For this work, we further need the bit of logic
        -- that says that producing more than one notarization share from
        -- the current node means we do not produce any finalization shares
        -- for that height.
        set_time time
        Context cfg@(my_id, subnet_id) p t <- get
        let actions = on_state_change cfg p t
            desc s =
              "========================================\n"
                ++ s
                ++ " set of possibilities:\n"
                ++ "\nPOOL:\n"
                ++ (if s == "NOT within" then dumpStr p else "")
                ++ "\n\nIDEAL:\n"
                ++ dumpStr actions
                ++ "\n\nREAL:\n"
                ++ dumpStr acc
        if isAnySubsetOf beq acc $!! actions
          then -- lift $ putStrLn (desc "Within")
            pure ()
          else do
            (validated_pool, _) <- get_pool
            let Just height = current_round validated_pool
            let Just rank =
                  block_maker_rank
                    validated_pool
                    height
                    my_id
                    subnet_id
            lift $ do
              putStrLn $
                "current_round = "
                  ++ show height
              putStrLn $
                "round_start = "
                  ++ show (round_start_time validated_pool height)
              putStrLn $
                "registry_version = "
                  ++ show (registry_version validated_pool height)
              putStrLn $
                "block_maker_rank = "
                  ++ show rank
              putStrLn $
                "block_maker_timeout = "
                  ++ show
                    ( block_maker_timeout
                        validated_pool
                        rank
                        height
                        subnet_id
                    )
              putStrLn (desc "NOT within")
              assertBool "" False
        [] <$ apply acc
replay _ = error "Log event trace must begin with FoundSubnets"

isAnySubsetOf :: ([a] -> [a] -> Bool) -> [a] -> [a] -> Bool
isAnySubsetOf eq xs ys =
  any (\zs -> any (eq xs) (subsequences zs)) (permutations ys)

-- (cd ~/dfinity/consensus-model/reference ; \
--  cabal test && cabal run exe:replay -- 100 ~/dl/log)

main :: IO ()
main = do
  args <- getArgs
  case args of
    ("-" : _) -> replay =<< readLogOutput stdin
    (file : _) -> replay =<< withFile file ReadMode readLogOutput
    _ -> putStrLn "usage: replay <LOGFILE|->"
