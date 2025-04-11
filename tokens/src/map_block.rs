use crate::pb::{
    sf::solana::spl::token::v1::{
        event::Type, Approve, Burn, CloseAccount, Event, Events, FreezeAccount, InitializeAccount,
        InitializeImmutableOwner, InitializeMint, InitializeMultisig, MintTo, Revoke, SetAuthority,
        SyncNative, ThawAccount, Transfer,
    },
    sol::transactions::v1::Transactions,
};
use substreams::{pb::substreams::Clock, skip_empty_output};
use substreams_solana::pb::sf::solana::r#type::v1::ConfirmedTransaction;
// use solana_sdk::pubkey::Pubkey;  // Import Pubkey from solana-sdk

// // Define the excluded program ID
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";


#[substreams::handlers::map]
fn map_block(
    params: String,
    clock: Clock,
    trxs: Transactions,
) -> Result<Events, substreams::errors::Error> {
    skip_empty_output();
    
    // Split the params string correctly to extract token contracts after 'token_contract:'
    let token_contracts: Vec<&str> = params
        .split(':')    // First split at ':'
        .skip(1)       // Skip the first part (before the colon)
        .collect::<Vec<&str>>()[0]   // Get the part after ':'
        .split(',')    // Split by comma to handle multiple token contracts
        .collect();    // Collect into a vector
    
    let block_height = clock.number;
    let block_timestamp = clock
        .timestamp
        .as_ref()
        .map(|t| t.seconds)
        .unwrap_or_default();

    let mut data: Vec<Event> = Vec::new();
    
    for confirmed_txn in trxs.transactions {
        if confirmed_txn.meta().is_none() {
            continue;
        }

        let tx_id = confirmed_txn.id();
        
        for (i, instruction) in confirmed_txn.walk_instructions().enumerate() {
            if instruction.program_id() != spl_token::ID && instruction.program_id().to_string() != TOKEN_2022_PROGRAM {
                continue;
            }


            // Use match to handle unpack errors gracefully
            let token_instruction = match spl_token::instruction::TokenInstruction::unpack(instruction.data()) {
                Ok(instr) => instr,
                Err(_) => continue, // Skip if unpack fails
            };
            
            let event = match Type::try_from((token_instruction, &instruction)) {
                Ok(event_type) => Event {
                    txn_id: tx_id.clone(),
                    block_height,
                    block_timestamp,
                    block_hash: clock.id.clone(),
                    instruction_index: i as u32,
                    r#type: Some(event_type),
                },
                Err(_) => continue,
            };

            // Iterate over each token contract and filter events
            for token_contract in &token_contracts {
                if event
                    .r#type
                    .as_ref()
                    .unwrap()
                    .is_for_token_contract(&confirmed_txn, token_contract)
                {
                    data.push(event.clone());
                }
            }
        }
    }

    Ok(Events { data })
}

impl Type {
    fn is_for_token_contract(&self, trx: &ConfirmedTransaction, contract: &str) -> bool {
        match self {
            Type::Transfer(Transfer { accounts, .. }) => {
                match &accounts.as_ref().unwrap().token_mint {
                    Some(token_mint) => token_mint == contract,
                    None => trx
                        .meta()
                        .unwrap()
                        .meta
                        .as_ref()
                        .unwrap()
                        .pre_token_balances
                        .iter()
                        .any(|token_balance| {
                            token_balance.mint == contract || token_balance.owner == contract
                        }),
                }
            }
            Type::InitializeMint(InitializeMint { accounts, .. }) => {
                accounts.as_ref().unwrap().mint == contract
            }
            Type::InitializeImmutableOwner(InitializeImmutableOwner { accounts: _, .. }) => {
                false
            }
            Type::InitializeAccount(InitializeAccount { accounts, .. }) => {
                accounts.as_ref().unwrap().mint == contract
            }
            Type::InitializeMultisig(InitializeMultisig { accounts: _, .. }) => {
                false
            }
            Type::Approve(Approve { accounts: _, .. }) => {
                false
            }
            Type::MintTo(MintTo { accounts, .. }) => accounts.as_ref().unwrap().mint == contract,
            Type::Revoke(Revoke { accounts: _, .. }) => {
                false
            }
            Type::SetAuthority(SetAuthority { accounts: _, .. }) => {
                false
            }
            Type::Burn(Burn { accounts, .. }) => accounts.as_ref().unwrap().mint == contract,
            Type::CloseAccount(CloseAccount { accounts: _, .. }) => {
                false
            }
            Type::FreezeAccount(FreezeAccount { accounts: _, .. }) => {
                false
            }
            Type::ThawAccount(ThawAccount { accounts: _, .. }) => {
                false
            }
            Type::SyncNative(SyncNative { accounts: _, .. }) => {
                false
            }
        }
    }
}
