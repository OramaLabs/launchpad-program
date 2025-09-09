// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

import { LAMPORTS_PER_SOL, PublicKey } from '@solana/web3.js';
import { Launchpad } from '../target/types/launchpad';
import * as anchor from "@coral-xyz/anchor";

type LaunchpadProgram = anchor.Program<Launchpad>;

module.exports = async function (provider: anchor.AnchorProvider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  // Add your deploy script here.
    const program = anchor.workspace.Launchpad as LaunchpadProgram;
  const params = {
    pointsSigner: provider.wallet.publicKey,
    pointsPerSol: new anchor.BN(1000), // 1000 points per SOL
    minTargetSol: new anchor.BN(0).mul(new anchor.BN(LAMPORTS_PER_SOL)),
    maxTargetSol: new anchor.BN(500).mul(new anchor.BN(LAMPORTS_PER_SOL)),
    minDuration: new anchor.BN(0),
    maxDuration: new anchor.BN(7 * 24 * 60 * 60),
    lbPair: PublicKey.default
  }

  // console.log((await program.account.globalConfig.all())[0].publicKey.toString());
  // console.log((await program.account.globalConfig.all())[0].account.minStakeDuration.toString());
  // console.log((await program.account.globalConfig.all())[0].account.minDuration.toString());
  // console.log((await program.account.globalConfig.all())[0].account.minTargetSol.toString());
  // console.log((await program.account.globalConfig.all())[0].account.maxDuration.toString());
  // console.log((await program.account.globalConfig.all())[0].account.maxTargetSol.toString());
  // console.log((await program.account.globalConfig.all())[0].account.admin.toString());
  // console.log((await program.account.globalConfig.all())[0].account.pointsSigner.toString());
  // console.log((await program.account.globalConfig.all())[0].account.totalLaunches.toString());
  // console.log((await program.account.globalConfig.all())[0].account.totalRaisedSol.toString());
  const tx = await program.methods.initializeConfig(params).accounts({
    admin: provider.wallet.publicKey
  }).signers([provider.wallet.payer!]).rpc({commitment:'confirmed'})

  console.log(tx)
};
