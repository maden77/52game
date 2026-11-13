use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::solana_program::program::invoke;

// PENTING: Biarkan ID ini dulu. Nanti setelah `anchor build` dan `anchor keys list`, 
// ganti ID ini dengan ID yang dihasilkan oleh terminal.
declare_id!("11111111111111111111111111111111"); 

#[program]
pub mod game_52 {
    use super::*;

    // Pemain 1 membuat game dan menebak angka (2 atau 5)
    pub fn create_game(ctx: Context<CreateGame>, guess: u8, wager: u64) -> Result<()> {
        require!(guess == 2 || guess == 5, GameError::InvalidGuess);
        require!(wager == 10_000_000, GameError::InvalidWager); // 0.01 SOL = 10,000,000 lamports

        let game = &mut ctx.accounts.game;
        game.player1 = ctx.accounts.player1.key();
        game.player1_guess = guess;
        game.wager = wager;
        game.status = GameStatus::WaitingForPlayer2;

        // Transfer 0.01 SOL dari Player 1 ke Escrow (akun game)
        let ix = system_instruction::transfer(
            &ctx.accounts.player1.key(),
            &game.key(),
            wager,
        );
        invoke(
            &ix,
            &[
                ctx.accounts.player1.to_account_info(),
                game.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        Ok(())
    }

    // Pemain 2 bergabung, memasang taruhan, dan sistem mengacak angka
    pub fn join_and_play(ctx: Context<JoinGame>, _random_seed: u64) -> Result<()> {
        let game = &mut ctx.accounts.game;
        require!(game.status == GameStatus::WaitingForPlayer2, GameError::GameNotAvailable);

        // Transfer 0.01 SOL dari Player 2 ke Escrow
        let ix = system_instruction::transfer(
            &ctx.accounts.player2.key(),
            &game.key(),
            game.wager,
        );
        invoke(
            &ix,
            &[
                ctx.accounts.player2.to_account_info(),
                game.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // --- LOGIKA PENGACAKAN SEDERHANA ---
        let clock = Clock::get()?;
        let generated_number = ((_random_seed + clock.slot) % 10) as u8; // Menghasilkan 0-9
        
        // Jika hasil modulo genap -> 2, ganjil -> 5
        let result: u8 = if generated_number % 2 == 0 { 2 } else { 5 };
        
        game.player2 = ctx.accounts.player2.key();
        game.result = result;
        game.status = GameStatus::Finished;

        // Tentukan pemenang
        let winner_key = if game.player1_guess == result {
            game.player1
        } else {
            game.player2
        };

        // Hitung Fee Developer (0.5% dari total pot 0.02 SOL)
        let total_pot = game.wager * 2;
        let dev_fee = total_pot * 5 / 1000; // 0.5%
        let winner_prize = total_pot - dev_fee;

        // Alamat Dev Anda (Pastikan ini sesuai dengan yang ada di HTML)
        let dev_address = ctx.accounts.dev_account.to_account_info();

        // Transfer Fee ke Dev
        **game.to_account_info().try_borrow_mut_lamports()? -= dev_fee;
        **dev_address.try_borrow_mut_lamports()? += dev_fee;

        // Transfer Sisa ke Pemenang
        let winner_account = if winner_key == game.player1 {
            ctx.accounts.player1.to_account_info()
        } else {
            ctx.accounts.player2.to_account_info()
        };

        **game.to_account_info().try_borrow_mut_lamports()? -= winner_prize;
        **winner_account.try_borrow_mut_lamports()? += winner_prize;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateGame<'info> {
    #[account(init, payer = player1, space = 8 + 32 + 1 + 8 + 1 + 32 + 1)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player1: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinGame<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player1: SystemAccount<'info>,
    #[account(mut)]
    pub player2: Signer<'info>,
    /// CHECK: Alamat dev diverifikasi secara hardcode
    #[account(mut, address = "3ninwAaR453Mipjv2FTx5xGbHoigRGxnc373PZrELZc5".parse::<Pubkey>().unwrap())]
    pub dev_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Game {
    pub player1: Pubkey,
    pub player1_guess: u8,
    pub wager: u64,
    pub status: GameStatus,
    pub player2: Pubkey,
    pub result: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum GameStatus {
    WaitingForPlayer2,
    Finished,
}

#[error_code]
pub enum GameError {
    #[msg("Tebakan harus 2 atau 5")]
    InvalidGuess,
    #[msg("Taruhan harus tepat 0.01 SOL")]
    InvalidWager,
    #[msg("Game tidak tersedia atau sudah selesai")]
    GameNotAvailable,
}