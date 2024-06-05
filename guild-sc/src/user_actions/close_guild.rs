multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CloseGuildModule {
    fn require_not_closing(&self) {
        let closing = self.guild_closing().get();
        require!(!closing, "Guild closing");
    }

    fn require_closing(&self) {
        let closing = self.guild_closing().get();
        require!(closing, "Guild not closing");
    }

    #[view(isGuildClosing)]
    #[storage_mapper("guildClosing")]
    fn guild_closing(&self) -> SingleValueMapper<bool>;
}
