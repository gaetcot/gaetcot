use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{
    self, Mint, MintTo, Token, TokenAccount, Transfer,
};
use anchor_lang::solana_program::clock::Clock;

declare_id!("9KwMVKWDcDhsAEinCdKYZpaitooRczJcgm6NTALhaM8f"); 
// ^ Cambia con la pubkey effettiva del tuo programma (ad es. generata da `anchor keys new`)

#[program]
pub mod survey_trend {
    use super::*;

    /// 1) INITIALIZE
    /// Crea il mint “SurveyTrend” e un suo treasury (o supply) iniziale.
    /// Inizializza la config con parametri base.
    /// Inizializza un registry dei survey (facoltativo).
    pub fn initialize(
        ctx: Context<Initialize>,
        total_supply: u64,
        bonus_percent: u8,
        min_holding_period: i64,
        halving_period: i64,
        max_surveys: u16,
    ) -> Result<()> {
        // Mint di "total_supply" token nel treasury
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.survey_trend_mint.to_account_info(),
                to: ctx.accounts.survey_trend_treasury.to_account_info(),
                authority: ctx.accounts.mint_authority.to_account_info(),
            },
        );
        token::mint_to(cpi_ctx, total_supply)?;

        // Inizializza la config (parametri di base)
        let config = &mut ctx.accounts.config;
        config.bonus_percent = bonus_percent;
        config.min_holding_period = min_holding_period;
        config.halving_period = halving_period;
        config.max_surveys = max_surveys;
        config.survey_trend_mint = ctx.accounts.survey_trend_mint.key();

        // Inizializza il registry se vuoi gestire i “Survey”
        let registry = &mut ctx.accounts.survey_registry;
        registry.surveys = Vec::new();

        msg!("✅ SurveyTrend mintato con supply: {}.", total_supply);
        Ok(())
    }

    /// 2) FUND TREASURY
    /// Permette di aggiungere token SurveyTrend al treasury.
    /// Trasferisce i token da un account esterno al treasury.
    pub fn fund_treasury(ctx: Context<FundTreasury>, amount: u64) -> Result<()> {
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.funder_account.to_account_info(),
                to: ctx.accounts.survey_trend_treasury.to_account_info(),
                authority: ctx.accounts.funder_authority.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, amount)?;

        emit!(TreasuryFunded {
            amount,
            new_treasury_balance: ctx.accounts.survey_trend_treasury.amount
        });
        Ok(())
    }

    /// 3) OPEN SURVEY
    /// Crea un nuovo survey (se non si supera max_survey e non c’è titolo duplicato).
    pub fn open_survey(ctx: Context<OpenSurvey>, title: String) -> Result<()> {
        let registry = &mut ctx.accounts.survey_registry;
        let config = &ctx.accounts.config;

        // Controllo limite
        if registry.surveys.len() as u16 >= config.max_surveys {
            return err!(CustomError::SurveyLimitReached);
        }

        // Controllo duplicato
        for s in &registry.surveys {
            if s.title == title {
                return err!(CustomError::DuplicateSurvey);
            }
        }

        let new_survey = Survey {
            title: title.clone(),
            creator: ctx.accounts.creator.key(),
            creation_timestamp: Clock::get()?.unix_timestamp,
        };
        registry.surveys.push(new_survey);

        emit!(SurveyCreated {
            creator: ctx.accounts.creator.key(),
            title,
        });
        Ok(())
    }

    /// 4) DISTRIBUTE REWARDS
    /// Esempio di distribuzione ricompense in base a snapshot / logica semplificata.
    /// In futuro potresti trasformarla in funzione che controlla se l’utente
    /// detiene (anche) un secondo token, per dargli un bonus extra (o mintare un nuovo token).
    pub fn distribute_rewards(ctx: Context<DistributeRewards>) -> Result<()> {
        let config = &ctx.accounts.config;
        let holder_balance = ctx.accounts.holder_account.amount;

        // Esempio: calcolo semplificato del bonus in base a config.bonus_percent
        // (In un sistema reale useresti un "weekly snapshot" e verificheresti min holding period).
        let bonus_amount = holder_balance
            .checked_mul(config.bonus_percent as u64).unwrap_or(0)
            .checked_div(100).unwrap_or(0);

        if bonus_amount > 0 {
            // Transfer dal treasury all’holder
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.survey_trend_treasury.to_account_info(),
                    to: ctx.accounts.holder_account.to_account_info(),
                    authority: ctx.accounts.treasury_authority.to_account_info(),
                },
            );
            token::transfer(cpi_ctx, bonus_amount)?;

            emit!(RewardsDistributed {
                holder: ctx.accounts.holder_account.owner,
                amount: bonus_amount,
            });
        }

        Ok(())
    }
}

// ----------------------------------------------------------------------------
// CONTEXTS
// ----------------------------------------------------------------------------

/// 1) CONTEXT: INITIALIZE
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Crea il Mint di “SurveyTrend”
    #[account(
        init,
        payer = payer,
        mint::decimals = 9,
        mint::authority = mint_authority
    )]
    pub survey_trend_mint: Account<'info, Mint>,

    /// Crea un "treasury" dove conserviamo i token
    #[account(
        init,
        payer = payer,
        token::mint = survey_trend_mint,
        token::authority = mint_authority
    )]
    pub survey_trend_treasury: Account<'info, TokenAccount>,

    /// Authority che firma la creazione del mint
    #[account(mut)]
    pub mint_authority: Signer<'info>,

    /// Account di configurazione
    #[account(
        init,
        payer = payer,
        space = 8 + Config::MAX_SIZE
    )]
    pub config: Account<'info, Config>,

    /// Account registry dove salviamo i surveys
    #[account(
        init,
        payer = payer,
        space = 8 + SurveyRegistry::MAX_SIZE
    )]
    pub survey_registry: Account<'info, SurveyRegistry>,

    /// Payer generico per i costi di creazione
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,
}

/// 2) CONTEXT: FUND_TREASURY
#[derive(Accounts)]
pub struct FundTreasury<'info> {
    #[account(mut)]
    pub survey_trend_treasury: Account<'info, TokenAccount>,
    #[account(mut)]
    pub funder_account: Account<'info, TokenAccount>,
    pub funder_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

/// 3) CONTEXT: OPEN_SURVEY
#[derive(Accounts)]
pub struct OpenSurvey<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub survey_registry: Account<'info, SurveyRegistry>,
    #[account(signer)]
    pub creator: AccountInfo<'info>,
}

/// 4) CONTEXT: DISTRIBUTE_REWARDS
#[derive(Accounts)]
pub struct DistributeRewards<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(mut)]
    pub survey_trend_treasury: Account<'info, TokenAccount>,
    pub treasury_authority: Signer<'info>,

    #[account(mut)]
    pub holder_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

// ----------------------------------------------------------------------------
// DATA ACCOUNTS
// ----------------------------------------------------------------------------

#[account]
pub struct Config {
    pub bonus_percent: u8,
    pub min_holding_period: i64,
    pub halving_period: i64,
    pub max_surveys: u16,

    pub survey_trend_mint: Pubkey,
}

impl Config {
    pub const MAX_SIZE: usize = 1 + 8 + 8 + 2 + 32;
}

#[account]
pub struct SurveyRegistry {
    pub surveys: Vec<Survey>,
}

impl SurveyRegistry {
    pub const MAX_SIZE: usize = 8_000; // dimensione massima per i surveys
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Survey {
    pub title: String,
    pub creator: Pubkey,
    pub creation_timestamp: i64,
}

// ----------------------------------------------------------------------------
// ERRORI ED EVENTI
// ----------------------------------------------------------------------------

#[error_code]
pub enum CustomError {
    #[msg("Survey limit reached.")]
    SurveyLimitReached,
    #[msg("Survey with same title already exists.")]
    DuplicateSurvey,
}

#[event]
pub struct TreasuryFunded {
    pub amount: u64,
    pub new_treasury_balance: u64,
}

#[event]
pub struct SurveyCreated {
    pub creator: Pubkey,
    pub title: String,
}

#[event]
pub struct RewardsDistributed {
    pub holder: Pubkey,
    pub amount: u64,
}
