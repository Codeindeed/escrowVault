use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
         close_account,transfer_checked, Mint, TokenAccount, TokenInterface, CloseAccount,
        TransferChecked,
    },
};
use crate::{state::Escrow, error::EscrowError};


#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Take<'info>{
    #[account(mut)]
    pub taker:Signer<'info>,
    #[account(mut)]
    pub maker:SystemAccount<'info>,
    #[account(mut, close = maker, seeds = [b"escrow", maker.key().as_ref(),seed.to_le_bytes().as_ref()], bump = escrow.bump,    
    has_one = maker @ EscrowError::InvalidMaker,
    has_one = mint_a @ EscrowError::InvalidMintA,
    has_one = mint_b @ EscrowError::InvalidMintB,)]
    pub escrow:Account<'info,Escrow>,
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
   
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    //programme
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
impl <'info> Take <'info>{
    fn transfer_to_maker(&mut self)->Result<()> {
        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from:self.taker_ata_b.to_account_info(),
                    to:self.maker_ata_b.to_account_info(),
                    authority:self.taker.to_account_info(),
                    mint:self.mint_b.to_account_info(),
                },
            ),
            self.escrow.recieve,
            self.mint_b.decimals,
        )?;
        Ok(())
    }

    fn withdraw_and_close(&mut self)->Result<()> {
      let seed_bytes = self.escrow.seed.to_le_bytes();
      let bump = [self.escrow.bump];
      let maker_key = self.maker.key();
      let seed_array: [&[u8]; 4] = [
          b"escrow",
          maker_key.as_ref(),
          &seed_bytes,
          &bump,
      ];
      let signer_seeds: &[&[&[u8]]] = &[&seed_array];
      transfer_checked(
        CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            TransferChecked{
                from:self.vault.to_account_info(),
                to:self.taker_ata_a.to_account_info(),
                authority:self.escrow.to_account_info(),
                mint:self.mint_a.to_account_info(),
            },
            signer_seeds,
        ),
        self.vault.amount,
        self.mint_a.decimals,
      )?;
        close_account(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                CloseAccount{
                    account:self.vault.to_account_info(),
                    authority:self.escrow.to_account_info(),
                    destination:self.maker.to_account_info(),
                },
                signer_seeds,
            )
        )?;
        Ok(())
    }
}

pub fn handler(ctx:Context<Take>)->Result<()> {
    ctx.accounts.transfer_to_maker()?;
    ctx.accounts.withdraw_and_close()?;
    Ok(())
}

