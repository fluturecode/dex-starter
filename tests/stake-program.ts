import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { StakeProgram } from "../target/types/stake_program";
import { PublicKey, Keypair, Connection, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo, mintToChecked } from "@solana/spl-token";


const tempKeypair = Keypair.generate();
console.log("temp address: " + tempKeypair.publicKey.toBase58().toString())
const connection = new Connection("http://127.0.0.1:8899", "confirmed");
const mintKeypair = Keypair.fromSecretKey(new Uint8Array([241, 54, 124, 229, 181, 1, 153, 130, 247, 143, 116, 195, 112, 73, 180, 65, 142, 1, 23, 59, 156, 160, 35, 153, 165, 37, 184, 146, 215, 162, 30, 195, 10, 50, 11, 235, 118, 208, 67, 186, 74, 76, 173, 247, 37, 67, 141, 132, 170, 17, 84, 105, 60, 0, 56, 122, 23, 196, 59, 85, 160, 13, 25, 178]));
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
const payer = provider.wallet as anchor.Wallet;
console.log("main key: " + payer.publicKey.toBase58().toString())


async function createNewMintToken() {
  await createMint(
    connection, // conneciton
    tempKeypair, // fee payer
    payer.publicKey, // mint authority
    payer.publicKey, // freeze authority (you can use `null` to disable it. when you disable it, you can't turn it on again)
    8, // decimals
    mintKeypair
  );
}

async function mintTokens() {
  let tokenAccount = await getOrCreateAssociatedTokenAccount(
    connection,
    tempKeypair,
    mintKeypair.publicKey,
    tempKeypair.publicKey,
  )
  await mintTo(
    connection, // connection
    tempKeypair, // fee payer
    mintKeypair.publicKey, // mint
    tokenAccount.address, // receiver (should be a token account)
    payer.payer, // mint authority
    1e8, // amount. if your decimals is 8, you mint 10^8 for 1 token.
  );
}

async function airdrop(addressToAirdrop: PublicKey) {
  const signature = await connection.requestAirdrop(
    addressToAirdrop,
    5*LAMPORTS_PER_SOL
  );
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  await connection.confirmTransaction({
      blockhash,
      lastValidBlockHeight,
      signature
    });
}

describe("stake-program", () => {
  // Configure the client to use the local cluster.
  const program = anchor.workspace.StakeProgram as Program<StakeProgram>;  

  it("Initialize Stake Pool", async () => {

    await airdrop(tempKeypair.publicKey)

    let [stakePool] = PublicKey.findProgramAddressSync(
      [Buffer.from("stake_pool")],
      program.programId
    );
    console.log('stake pool: ' + stakePool.toBase58().toString())

    const tx = await program.methods
      .initStakepool()
      .accounts({
        signer: tempKeypair.publicKey,
        mint: mintKeypair.publicKey,
        stakePoolTokenAccount: stakePool,
      })
      .signers([tempKeypair])
      .rpc();
  });

  it("Stake on a stake pool", async () => {
    await airdrop(tempKeypair.publicKey)

    //await createNewMintToken()
    await mintTokens()

    let [stakeInfo] = PublicKey.findProgramAddressSync(
      [Buffer.from("stake_info"), tempKeypair.publicKey.toBuffer()],
      program.programId
    );

    const [stakerStakeTokenAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("token"), tempKeypair.publicKey.toBuffer()],
      program.programId
    )

    let stakerTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      tempKeypair, // Payer
      mintKeypair.publicKey, // Mint
      tempKeypair.publicKey // Owner
    )

    const tx = await program.methods
      .stake(new anchor.BN(1))
      .signers([tempKeypair])
      .accounts({
        stakeInfo: stakeInfo,
        signer: tempKeypair.publicKey,
        mint: mintKeypair.publicKey,
        stakerStakeTokenAccount: stakerStakeTokenAccount,
        stakerTokenAccount: stakerTokenAccount.address,
      })
      .rpc();
    console.log("Your stake transaction signature", tx);
  });

  it("Unstake", async () => {
    await airdrop(tempKeypair.publicKey)

    //await createNewMintToken()
    await mintTokens()

    let [stakeInfo] = PublicKey.findProgramAddressSync(
      [Buffer.from("stake_info"), tempKeypair.publicKey.toBuffer()],
      program.programId
    );

    const [stakerStakeTokenAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("token"), tempKeypair.publicKey.toBuffer()],
      program.programId
    )

    let stakerTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      tempKeypair, // Payer
      mintKeypair.publicKey, // Mint
      tempKeypair.publicKey // Owner
    )

    const [stakePoolTokenAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("stake_pool")],
      program.programId
    )
    console.log("!")
    await mintTo(
      connection, // connection
      tempKeypair, // fee payer
      mintKeypair.publicKey, // mint
      stakePoolTokenAccountAddress, // receiver (should be a token account)
      payer.payer, // mint authority
      10000e8, // amount. if your decimals is 8, you mint 10^8 for 1 token.
    );
    console.log("after")

    const tx = await program.methods
      .unstake()
      .signers([tempKeypair])
      .accounts({
        stakeInfo: stakeInfo,
        signer: tempKeypair.publicKey,
        mint: mintKeypair.publicKey,
        stakerStakeTokenAccount: stakerStakeTokenAccount,
        stakerTokenAccount: stakerTokenAccount.address,
        stakePoolTokenAccount: stakePoolTokenAccountAddress,
      })
      .rpc();
    console.log("Your stake transaction signature", tx);
  });
});

