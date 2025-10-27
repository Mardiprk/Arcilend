use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("CfuTSUUVQnPrMjSLwSoERGaDrAojWBfZ4UhCWAUNxuff");

#[program]
pub mod arcilend {
    use super::*;

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        interest_rate: u16, collateral_ratio: u16,
        liquidation_threshold: u16
    ) -> Result<()> {
        require!(
            collateral_ratio >= MIN_COLLATERAL_RATIO && collateral_ratio <= MAX_COLLATERAL_RATIO,
            ArciLendError::InvalidCollateralRatio);
        require!(
            interest_rate <= BASIS_POINTS,
            ArciLendError::InvalidInterestRate
        );
        require!(
            liquidation_threshold < collateral_ratio,
            ArciLendError::InvalidLiquidationThreshold
        );

        let lending_pool = &mut ctx.accounts.lending_pool;
        
        lending_pool.authority = ctx.accounts.authority.key();
        lending_pool.total_deposits = 0;
        lending_pool.total_borrowed = 0;
        lending_pool.interest_rate = interest_rate;
        lending_pool.collateral_ratio = collateral_ratio;
        lending_pool.liquidation_threshold = liquidation_threshold;
        lending_pool.arcium_mcp_pubkey = ctx.accounts.arcium_mpc_pubkey.key();
        lending_pool.oracle_feed = ctx.accounts.oracle_feed.key();
        lending_pool.bump = ctx.bumps.lending_pool;
        lending_pool.utilization_rate = 0;
        lending_pool.total_fees = 0;

        msg!("Lending pool initialized!");
        msg!("Interest Rate {}bps", interest_rate);
        msg!("Collateral Rate {}", collateral_ratio / 100);

        Ok(())
    }
    pub fn deposit_collateral(ctx: Context<DepositCollateral>, anount: u64) -> Result<()>{
        let user_account = &ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;

        Ok(())
    }

    pub fn request_credit_score(ctx: Context<RequestCreditScore>) -> Result<()>{
        let user_account = &ctx.accounts.user_account;

        Ok(())
    }

    pub fn update_credit_score(ctx: Context<UpdateCreditScore>, encrypted_score: [u8;32], risk_ajusted_ltv: u16) -> Result<()>{
        let user_account = &ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;

        Ok(())
    }

    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()>{
        let user_account = &ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;
        let loan = &mut ctx.accounts.loan;
        let clock = Clock::get()?;

        Ok(())
    }

    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()>{
let user_account = &ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;
        let loan = &mut ctx.accounts.loan;
        let clock = Clock::get()?;

        Ok(())
    }

    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()>{
        let user_account = &ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;
        
        Ok(())
    }

    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()>{
        let lending_pool = &mut ctx.accounts.lending_pool;
        let loan = &mut ctx.accounts.loan;
        let clock = Clock::get()?;

        Ok(())
    }

    pub fn accure_interest(ctx: Context<AccrueInterest>) -> Result<()>{
        let loan = &mut ctx.accounts.loan;
        let clock = Clock::get()?;

        loan.accrue_interest(clock.unix_timestamp);

        msg!("Interest accrued: {} lamports", loan.accrued_interest);

        Ok(())
    }
}

/// ---- Accounts ----

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + LendingPool::INIT_SPACE,
        seeds = [LENDING_POOL_SEED],
        bump
    )]
    pub lending_pool: Account<'info, LendingPool>,
    // CHECK: Authorized MPC node
    pub arcium_mpc_pubkey: AccountInfo<'info>,
    // CHECK: Orcale Feed
    pub oracle_feed: AccountInfo<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        seeds = [LENDING_POOL_SEED],
        bump = lending_pool.bump
    )]
    pub lending_pool: Account<'info, LendingPool>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserAccount::INIT_SPACE,
        seeds = [USER_ACCOUNT_SEED, user.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,

    pub system_program: Program<'info, System>

}

#[derive(Accounts)]
pub struct RequestCreditScore<'info> {
    #[account(
        seeds = [LENDING_POOL_SEED],
        bump
    )]
    pub lending_pool: Account<'info, LendingPool>,

    #[account(
        mut,
        seeds = [USER_ACCOUNT_SEED, user.key().as_ref()],
        bump = user_account.bump,
        constraint = user_account.owner == user.key()
    )]
    pub user_account: Account<'info, UserAccount>,

    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateCreditScore<'info> {
    #[account(
        seeds = [LENDING_POOL_SEED],
        bump
    )]
    pub lending_pool: Account<'info, LendingPool>,

    #[account(
        mut,
        seeds = [USER_ACCOUNT_SEED, user_account.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    pub mpc_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,

    #[account(
        seeds = [LENDING_POOL_SEED],
        bump
    )]
    pub lending_pool: Account<'info, LendingPool>,

    #[account(
        mut,
        seeds = [USER_ACCOUNT_SEED, borrower.key().as_ref()],
        bump = user_account.bump,
        constraint = user_account.owner == borrower.key()
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        init,
        payer = borrower,
        space = Loan::INIT_SPACE,
        seeds = [LOAN_SEED, borrower.key().as_ref()],
        bump
    )]
    pub loan: Account<'info, Loan>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct Repay<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,

    #[account(
        seeds = [LENDING_POOL_SEED],
        bump = lending_pool.bump
    )]
    pub lending_pool: Account<'info, LendingPool>,

    #[account(
        mut,
        seeds = [USER_ACCOUNT_SEED, loan.borrower.as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        mut,
        seeds = [LOAN_SEED, loan.borrower.key().as_ref()],
        bump = loan.bump
    )]
    pub loan: Account<'info, Loan>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [LENDING_POOL_SEED],
        bump = lending_pool.bump
    )]
    pub lending_pool: Account<'info, LendingPool>,

    #[account(
        mut,
        seeds = [USER_ACCOUNT_SEED, user.key().as_ref()],
        bump = user_account.bump,
        constraint = user_account.owner == user.key()
    )]
    pub user_account: Account<'info, UserAccount>,

    pub system_program: Program<'info, System>

}

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(
        mut,
        seeds = [LENDING_POOL_SEED],
        bump = lending_pool.bump
    )]
    pub lending_pool: Account<'info, LendingPool>,

    #[account(
        mut,
        seeds = [USER_ACCOUNT_SEED, loan.borrower.as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        mut,
        seeds = [LOAN_SEED, loan.borrower.as_ref()],
        bump = loan.bump
    )]
    pub loan: Account<'info, Loan>,

    #[account(mut)]
    pub liquidator: Signer<'info>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct AccrueInterest<'info> {
    #[account(
        mut,
        seeds = [LOAN_SEED, loan.borrower.as_ref()],
        bump = loan.bump
    )]
    pub loan: Account<'info, Loan>
}

/// ---- Lending Pool Struct ----

#[account]
#[derive(InitSpace)]
pub struct LendingPool{
    pub authority: Pubkey,
    pub total_deposits: u64,
    pub total_borrowed: u64,
    pub interest_rate: u16,
    pub collateral_ratio: u16,
    pub liquidation_threshold: u16,
    pub arcium_mcp_pubkey: Pubkey,
    pub oracle_feed: Pubkey,
    pub bump: u8,
    pub utilization_rate: u16,
    pub total_fees: u64,
}

impl LendingPool{
    pub fn calculate_utilization(&mut self){
        if self.total_deposits == 0 {
            self.utilization_rate = 0;
        }else {
            self.utilization_rate = ((self.total_borrowed as u128 * 10000) / self.total_deposits as u128) as u16 
        }
    }

    pub fn get_curent_interest_rate(&self) -> u16 {
        let base_rate = self.interest_rate;
        let optimal = 8000;

        if self.utilization_rate <= optimal {
            base_rate
        }else{
            let excess = self.utilization_rate - optimal;
            base_rate + (excess * 5)
        }
    }
}

/// ---- USer Account Struct ----

#[account]
#[derive(InitSpace)]
pub struct UserAccount{
    pub owner: Pubkey,
    pub collateral_borrowed: u64,
    pub amount_borrowed: u64,
    pub last_update: i64,
    pub loan_count: u8,
    pub encrypted_credit_score: [u8; 32],
    pub rist_adjusted_ltv: u16,
    pub successful_payments: u16,
    pub defaults: u16,
    pub bump: u8
}

impl UserAccount {
    pub fn is_liquidatable(&self, _price: u64, liquidation_threshold: u16) -> bool {
        if self.amount_borrowed == 0 {
            return false;
        }

        let collateral_value = self.collateral_borrowed;
        let debt_threshold = (self.amount_borrowed as u128 * liquidation_threshold as u128) / 10000;

        collateral_value < debt_threshold as u64
    }
}

/// ---- LOAN Struct ----

#[account]
#[derive(InitSpace)]
pub struct Loan {
    pub borrower: Pubkey,
    pub user_account: Pubkey,
    pub collateral_amount: u64,
    pub borrowed_amount: u64,
    pub interest_rate: u16,
    pub start_time: i64,
    pub last_accrual: i64,
    pub accrued_interest: u64,
    pub is_liquidated: bool,
    pub bump: u8,
}

impl Loan {
    pub fn accrue_interest(&mut self, current_time: i64) {
        let time_elapsed = (current_time - self.last_accrual) as u64;
        let seconds_per_year = 365 * 24 * 60 * 60;
        
        let interest = (self.borrowed_amount as u128 * self.interest_rate as u128 * time_elapsed as u128)
            / (seconds_per_year as u128 * 10000);
        
        self.accrued_interest += interest as u64;
        self.last_accrual = current_time;
    }

    pub fn total_owed(&self) -> u64 {
        self.borrowed_amount + self.accrued_interest
    }
}

pub const LENDING_POOL_SEED: &[u8] = b"lending_pool";
pub const USER_ACCOUNT_SEED: &[u8] = b"user_account";
pub const LOAN_SEED: &[u8] = b"loan";

pub const MIN_COLLATERAL_RATIO: u16 = 12000;
pub const MAX_COLLATERAL_RATIO: u16 = 30000;
pub const LIQUIDATION_BONUS: u16 = 500;
pub const MIN_LTV: u16 = 5000;
pub const MAX_LTV: u16 = 8000;
pub const BASIS_POINTS: u16 = 10000;

/// ---- ERRORs ----
#[error_code]
pub enum ArciLendError {
    #[msg("Invalid collateral ratio")]
    InvalidCollateralRatio,
    #[msg("Invalid interest rate")]
    InvalidInterestRate,
    #[msg("Insufficient collateral")]
    InsufficientCollateral,
    #[msg("Loan not liquidatable")]
    LoanNotLiquidatable,
    #[msg("Undercollateralized")]
    Undercollateralized,
    #[msg("Active loans exist")]
    ActiveLoansExist,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Already liquidated")]
    AlreadyLiquidated,
    #[msg("Unauthorized MPC update")]
    UnauthorizedMPCUpdate,
    #[msg("Invalid liquidation threshold")]
    InvalidLiquidationThreshold,
    #[msg("Invalid credit score")]
    InvalidCreditScore,
    #[msg("Exceeds risk-adjusted LTV")]
    ExceedsRiskAdjustedLTV,
}