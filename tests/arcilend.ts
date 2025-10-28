import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Arcilend } from "../target/types/arcilend";
import { PublicKey, Keypair } from "@solana/web3.js";

describe("arcilend", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.arcilend as Program<Arcilend>;

  const mpcNode = Keypair.generate();
  const oracleFeed = Keypair.generate();

  console.log("initializing ArciLend Pool...");
  console.log("MPC Node:", mpcNode.publicKey.toString());
  console.log("Oracle Feed:", oracleFeed.publicKey.toString());

  const [lendingPoolPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("lending_pool")],
    program.programId,
  );

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initializePool(
      500, // 5% interest rate
      15000, // 150% collateral ratio
      12000 // 120% liquidation threshold
    ).accounts({
      authority: provider.wallet.publicKey,
      lendingPool: lendingPoolPDA,
      arciumMpcPubkey: mpcNode.publicKey,
      oracleFeed: oracleFeed.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
    }).rpc();

  console.log("✅ Pool initialized!");
  console.log("Transaction:", tx);
  console.log("Lending Pool PDA:", lendingPoolPDA.toString());

  const pool = await program.account.lendingPool.fetch(lendingPoolPDA);
  console.log("\n📊 Pool State:");
  console.log("Authority:", pool.authority.toString());
  console.log("Interest Rate:", pool.interestRate, "bps");
  console.log("Collateral Ratio:", pool.collateralRatio / 100, "%");

  console.log("\n💾 Save these for your .env.local:");
  console.log(`NEXT_PUBLIC_PROGRAM_ID=${program.programId.toString()}`);
  console.log(`NEXT_PUBLIC_LENDING_POOL=${lendingPoolPDA.toString()}`);
  console.log(`NEXT_PUBLIC_MPC_NODE=${mpcNode.publicKey.toString()}`);

  });
});
