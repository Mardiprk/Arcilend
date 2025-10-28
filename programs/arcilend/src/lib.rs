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
    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        let lending_pool_info = ctx.accounts.lending_pool.to_account_info();
        let user_account = &mut ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;
    
        // Initialize user account if first time
        if user_account.owner == Pubkey::default() {
            user_account.owner = ctx.accounts.user.key();
            user_account.collateral_deposited = 0;
            user_account.amount_borrowed = 0;
            user_account.last_update = Clock::get()?.unix_timestamp;
            user_account.loan_count = 0;
            user_account.encrypted_credit_score = [0u8; 32];
            user_account.risk_adjusted_ltv = 5000;
            user_account.successful_repayments = 0;
            user_account.defaults = 0;
            user_account.bump = ctx.bumps.user_account;
        }
    
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: lending_pool_info,
            },
        );
        system_program::transfer(cpi_context, amount)?;
    
        user_account.collateral_deposited += amount;
        lending_pool.total_deposits += amount;
        lending_pool.calculate_utilization();
    
        msg!("Deposited {} lamports", amount);
        msg!("Total collateral: {}", user_account.collateral_deposited);
    
        Ok(())
    }

    pub fn request_credit_score(ctx: Context<RequestCreditScore>) -> Result<()>{
        let user_account = &ctx.accounts.user_account;

        emit!(
            CreditScoreRequested{
                user: user_account.owner.key(),
                collateral_deposited: user_account.collateral_deposited,
                amount_borrowed: user_account.amount_borrowed,
                successful_repayments: user_account.successful_repayments,
                defaults: user_account.defaults,
                timestamp: Clock::get()?.unix_timestamp,
            }
        );
        
        msg!("🔐 Credit score calculation requested");
        msg!("User: {}", user_account.owner);

        Ok(())
    }

    pub fn update_credit_score(ctx: Context<UpdateCreditScore>, encrypted_score: [u8;32], risk_adjusted_ltv: u16) -> Result<()>{
        let user_account = &mut ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;
        
        require!(
            ctx.accounts.mpc_authority.key() == lending_pool.arcium_mcp_pubkey,
            ArciLendError::UnauthorizedMPCUpdate
        );
        require!(
            risk_adjusted_ltv >= MIN_LTV && risk_adjusted_ltv <= MAX_LTV,
            ArciLendError::InvalidCreditScore
        );

        user_account.encrypted_credit_score = encrypted_score;
        user_account.risk_adjusted_ltv = risk_adjusted_ltv;

        msg!("✅ Credit score updated via MPC!");
        msg!("Risk-adjusted LTV: {}%", risk_adjusted_ltv / 100);
        
        Ok(())
    }

    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()>{
        let user_account = &ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;
        let loan = &mut ctx.accounts.loan;
        let clock = Clock::get()?;

        let collateral_value = user_account.collateral_deposited;
        let max_borrow = (collateral_value as u128 * user_account.risk_adjusted_ltv as u128) / BASIS_POINTS as u128;

        require!(
            amount <= max_borrow as u64,
            ArciLendError::ExceedsRiskAdjustedLTV
        );

        let new_total_borrowed = user_account.amount_borrowed + amount;
        let thereshold_value = (collateral_value as u128 * lending_pool.collateral_ratio as u128) / BASIS_POINTS as u128;
        
        require!(
            new_total_borrowed <= thereshold_value as u64,
            ArciLendError::Undercollateralized
        );


        let base_rate = lending_pool.get_curent_interest_rate();
        let risk_premium = if user_account.risk_adjusted_ltv > 7000 {
            0   // good credit no premium
        } else {
            200 // + 2% for lower credit score
        };

        let personalized_rate = base_rate + risk_premium;
        
        // initialize loan
        loan.borrower = ctx.accounts.borrower.key();
        loan.user_account = user_account.key();
        loan.collateral_amount = user_account.collateral_deposited;
        loan.borrowed_amount = amount;
        loan.interest_rate = personalized_rate;
        loan.start_time = clock.unix_timestamp;
        loan.last_accrual = clock.unix_timestamp;
        loan.accrued_interest = 0;
        loan.is_liquidated = false;
        loan.bump = ctx.bumps.loan;

        // Transfer borrowed amount to user
        **lending_pool.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.borrower.to_account_info().try_borrow_mut_lamports()? += amount;
        
        let user_account = &mut ctx.accounts.user_account;

        user_account.amount_borrowed += amount;
        user_account.loan_count += 1;
        user_account.last_update = clock.unix_timestamp;

        lending_pool.total_borrowed += amount;
        lending_pool.calculate_utilization();

        msg!("✅ Loan created!");
        msg!("Borrowed: {} lamports at {}bps", amount, personalized_rate);

        Ok(())
    }

    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()>{
        let user_account = &mut ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;
        let loan = &mut ctx.accounts.loan;
        let clock = Clock::get()?;

        loan.accrue_interest(clock.unix_timestamp);

        let total_owed = loan.total_owed();
        let repay_amount = amount.min(total_owed);

        require!(repay_amount > 0, ArciLendError::InsufficientBalance);

        **ctx.accounts.borrower.to_account_info().try_borrow_mut_lamports()? -= repay_amount;
        **lending_pool.to_account_info().try_borrow_mut_lamports()? += repay_amount;

        if repay_amount >= loan.accrued_interest {
            let principal_payment = repay_amount - loan.accrued_interest;
            loan.accrued_interest = 0;
            loan.borrowed_amount -= principal_payment;
        } else {
            loan.accrued_interest -= repay_amount;
        }

        user_account.amount_borrowed = user_account.amount_borrowed.saturating_sub(repay_amount);

        if loan.borrowed_amount == 0 {
            user_account.successful_repayments += 1;
        }

        lending_pool.total_borrowed = lending_pool.total_borrowed.saturating_sub(repay_amount);
        lending_pool.calculate_utilization();

        msg!("Repaid {} lamports", repay_amount);

        Ok(())
    }

    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()>{
        let user_account = &mut ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;
        
        require!(user_account.amount_borrowed == 0, ArciLendError::ActiveLoansExist);
        require!(amount <= user_account.collateral_deposited, ArciLendError::InsufficientBalance);
        
        **lending_pool.to_account_info().try_borrow_mut_lamports()? -= amount;
        **user_account.to_account_info().try_borrow_mut_lamports()? += amount;

        user_account.collateral_deposited -= amount;
        lending_pool.total_deposits -= amount;
        lending_pool.calculate_utilization();

        msg!("Withdrew {} lamports", amount);

        Ok(())
    }

    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()>{
        let user_account = &mut ctx.accounts.user_account;
        let lending_pool = &mut ctx.accounts.lending_pool;
        let loan = &mut ctx.accounts.loan;
        let clock = Clock::get()?;

        require!(!loan.is_liquidated, ArciLendError::AlreadyLiquidated);

        loan.accrue_interest(clock.unix_timestamp);

        let collateral_price = 1_000_000_000;
        
        require!(
            user_account.is_liquidatable(collateral_price, lending_pool.liquidation_threshold),
            ArciLendError::LoanNotLiquidatable
        );

        let total_dept = loan.total_owed();
        let collateral_to_seize = loan.collateral_amount;
        let bonus = (collateral_to_seize as u128 * LIQUIDATION_BONUS as u128) / BASIS_POINTS as u128;
        let total_reward = collateral_to_seize + bonus as u64;

        **ctx.accounts.liquidator.to_account_info().try_borrow_mut_lamports()? -= total_dept;
        **lending_pool.to_account_info().try_borrow_mut_lamports()? += total_dept;

        **ctx.accounts.liquidator.to_account_info().try_borrow_mut_lamports()? -= total_reward;
        **lending_pool.to_account_info().try_borrow_mut_lamports()? += total_reward;

        loan.is_liquidated = true;
        user_account.amount_borrowed -= loan.borrowed_amount;
        user_account.collateral_deposited -= collateral_to_seize;
        user_account.defaults += 1;

        lending_pool.total_borrowed -= loan.borrowed_amount;
        lending_pool.total_deposits -= collateral_to_seize;
        lending_pool.calculate_utilization();

        msg!("Liquidation successful!");
        msg!("Debt: {}, seized: {}, Bonus: {}", total_dept, collateral_to_seize, bonus);

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
    /// CHECK: This is the authorized Arcium MPC node public key that will provide credit score updates
    pub arcium_mpc_pubkey: AccountInfo<'info>,
    /// CHECK: This is the oracle feed address (Pyth/Switchboard) for price data
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

/// ---- Event ----
#[event]
pub struct CreditScoreRequested {
    pub user: Pubkey,
    pub collateral_deposited: u64,
    pub amount_borrowed: u64,
    pub successful_repayments: u16,
    pub defaults: u16,
    pub timestamp: i64,
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
    pub collateral_deposited: u64,
    pub amount_borrowed: u64,
    pub last_update: i64,
    pub loan_count: u8,
    pub encrypted_credit_score: [u8; 32],
    pub risk_adjusted_ltv: u16,
    pub successful_repayments: u16,
    pub defaults: u16,
    pub bump: u8
}

impl UserAccount {
    pub fn is_liquidatable(&self, _price: u64, liquidation_threshold: u16) -> bool {
        if self.amount_borrowed == 0 {
            return false;
        }

        let collateral_value = self.collateral_deposited;
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