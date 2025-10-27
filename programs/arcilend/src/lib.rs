use anchor_lang::prelude::*;

declare_id!("CfuTSUUVQnPrMjSLwSoERGaDrAojWBfZ4UhCWAUNxuff");

#[program]
pub mod arcilend {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
