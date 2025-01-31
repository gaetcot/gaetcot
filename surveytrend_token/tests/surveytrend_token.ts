import * as anchor from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

describe("surveytrend_token", () => {
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);
  const program = anchor.workspace.SurveytrendToken;

  it("Initialize Token!", async () => {
    const mint = anchor.web3.Keypair.generate();
    await program.rpc.initialize({
      accounts: {
        mint: mint.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [mint],
    });

    console.log("Token Minted:", mint.publicKey.toString());
  });
});
