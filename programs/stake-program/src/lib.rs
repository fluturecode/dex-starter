use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer, transfer}
};
use solana_program::clock::Clock;

declare_id!("8eMvEVSvPDNj2XdMcumtk1Ud86Wi5HHZzymSoVBUKKCJ");

pub mod constants {
    pub const STAKE_POOL_SEED: &[u8] = b"stake_pool"; // Seed for the stake pool token account PDA.
    pub const STAKE_INFO_SEED: &[u8] = b"stake_info"; // Seed for the player's stake account PDA.
    pub const TOKEN_SEED: &[u8] = b"token"; // Seed for the player's stake token account PDA.
}

#[program]
pub mod stake_program {

    use super::*;

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {

        let clock = Clock::get()?;
        let stake_info = &mut ctx.accounts.stake_info;

        if stake_info.is_staked {
            return Err(ErrorCode::IsStaked.into());
        }

        if amount <= 0 {
            return Err(ErrorCode::NoTokens.into());
        }

        stake_info.is_staked = true;
        stake_info.stake_at_slot = clock.slot;

        let amount_to_stake = (amount)
            .checked_mul(10u64.pow(ctx.accounts.mint.decimals as u32))
            .unwrap();

        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.staker_token_account.to_account_info(),
                    to: ctx.accounts.staker_stake_token_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                },
            ),
            amount_to_stake,
        )?;

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {

        let stake_info = &mut ctx.accounts.stake_info;
        let clock = Clock::get()?;

        if !stake_info.is_staked {
            return Err(ErrorCode::NotStaked.into());
        }

        let slots_passed = clock.slot - stake_info.stake_at_slot;

        let stake_amount = ctx.accounts.staker_stake_token_account.amount;
        msg!("stake amount");
        msg!(&stake_amount.to_string());

        let bump = *ctx.bumps.get("stake_pool_token_account").unwrap();
        let signer: &[&[&[u8]]] = &[&[constants::STAKE_POOL_SEED, &[bump]]];

        let amount = (slots_passed as u64)
            .checked_mul(10u64.pow(ctx.accounts.mint.decimals as u32))
            .unwrap();

        // Transfer rewards to player's token account from vault token account.
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.stake_pool_token_account.to_account_info(),
                    to: ctx.accounts.staker_token_account.to_account_info(),
                    authority: ctx.accounts.stake_pool_token_account.to_account_info(),
                },
                signer,
            ),
            amount,
        )?;

        // Player stake token account PDA signer
        let staker = ctx.accounts.signer.key();
        let bump = *ctx.bumps.get("staker_stake_token_account").unwrap();
        let signer: &[&[&[u8]]] = &[&[constants::TOKEN_SEED, staker.as_ref(), &[bump]]];

        // Transfer staked tokens from player's stake token account to player's token account.
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.staker_stake_token_account.to_account_info(),
                    to: ctx.accounts.staker_token_account.to_account_info(),
                    authority: ctx.accounts.staker_stake_token_account.to_account_info(),
                },
                signer,
            ),
            ctx.accounts.staker_stake_token_account.amount, // Transfer all tokens from the player's stake token account.
        )?;

        // Update the player_stake_account status and timestamp.
        stake_info.is_staked = false;
        stake_info.stake_at_slot = clock.slot;

        Ok(())
    }

    pub fn init_stakepool(_ctx: Context<StakePool>) -> Result<()> {
        Ok(())
    }

}

#[derive(Accounts)]
pub struct StakePool<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    // The vault token account that will hold the tokens rewarded to the player for staking.
    // The same PDA is used as both the address of the token account and the "owner" of token account.
    #[account(
        init_if_needed,
        seeds = [constants::STAKE_POOL_SEED],
        bump,
        payer = signer,
        token::mint = mint,
        token::authority = stake_pool_token_account,
    )]
    pub stake_pool_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(
        init_if_needed,
        seeds = [constants::STAKE_INFO_SEED, signer.key().as_ref()],
        bump,
        payer = signer,
        space = 8 + std::mem::size_of::<StakeInfo>()
    )]
    stake_info: Account<'info, StakeInfo>,

    // The player's stake token account, storing the tokens staked by the player.
    // The same PDA is used as both the address of the token account and the "owner" of token account.
    // Tokens are transferred from the player's token account to this account.
    #[account(
        init_if_needed,
        seeds = [constants::TOKEN_SEED, signer.key.as_ref()],
        bump,
        payer = signer,
        token::mint = mint,
        token::authority = staker_stake_token_account,
    )]
    pub staker_stake_token_account: Account<'info, TokenAccount>,

    // The player's associated token account for the token they are staking.
    // Tokens are transferred from this account to the player's stake token account.
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = signer
    )]
    pub staker_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    // The player's stake account, storing the player's staking status and timestamp.
    #[account(
        mut,
        seeds = [constants::STAKE_INFO_SEED, signer.key.as_ref()],
        bump,
    )]
    pub stake_info: Account<'info, StakeInfo>,

    // The player's stake token account, storing the tokens staked by the player.
    // Tokens transferred from this account back to the player's token account.
    #[account(
        mut,
        seeds = [constants::TOKEN_SEED, signer.key.as_ref()],
        bump,
    )]
    pub staker_stake_token_account: Account<'info, TokenAccount>,

    // The player's associated token account for the token they are staking.
    // Tokens transferred into this account from the vault token account and stake token account.
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = signer
    )]
    pub staker_token_account: Account<'info, TokenAccount>,

    // The vault token account that will hold the tokens rewarded to the player for staking.
    // Tokens transferred from this account to the player's token account.
    #[account(
        mut,
        seeds = [constants::STAKE_POOL_SEED],
        bump,
    )]
    pub stake_pool_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[account]
pub struct StakeInfo {
    pub stake_at_slot: u64,
    pub is_staked: bool,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Tokens Already Staked")]
    IsStaked,
    #[msg("Tokens Not Staked Yet")]
    NotStaked,
    #[msg("No Tokens to stake")]
    NoTokens,
}