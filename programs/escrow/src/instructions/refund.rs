use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        Mint,
        TokenAccount,
        TokenInterface,
        close_account,
        transfer_checked,
        CloseAccount,
        TransferChecked,
    },
};

use crate::state::Escrow;

#[derive(Accounts)]
pub struct Refund<'info> {

    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority= maker,
        associated_token::token_program=token_program
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close = maker, // if account closes it will send remaining lamports to maker
        has_one= mint_a, // ensuring that the escrow account is associated with the mint_a
        has_one = maker, // to make sure that the escrow account is owned by the maker
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        associated_token::mint = mint_a, // ensuring that the vault is associated with mint_a
        associated_token::authority = escrow, // to make sure that escrow owns the vault
        associated_token::token_program = token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    // programs required for the transaction
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Refund<'info> {
    /// this function is used to refund the tokens to maker and close the escrow account.
    pub fn refund_and_close(&mut self) -> Result<()> {
        // defining the required seeds for the escrow account
        let seeds: [&[&[u8]]; 1] = [
            &[
                b"escrow",
                self.maker.to_account_info().key.as_ref(),
                &self.escrow.seed.to_le_bytes()[..],
                &[self.escrow.bump],
            ],
        ];

        // accounts that are involved in transfe
        let transfer_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            authority: self.escrow.to_account_info(),
            to: self.maker_ata_a.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            transfer_accounts,
            &seeds
        );

        // transfer check for better validation and verification
        transfer_checked(cpi_ctx, self.vault.amount, self.mint_a.decimals)?;

        // closing the escrow account and sending the remaining lamports to maker
        let close_accounts = CloseAccount {
            account: self.vault.to_account_info(), // the vault account to be closed
            authority: self.escrow.to_account_info(), // the escrow account authority
            destination: self.maker.to_account_info(), // the destination account where remaining lamports will be sent
        };

        let close_cpi = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            close_accounts,
            &seeds
        );

        close_account(close_cpi)?;
        Ok(())
    }
}