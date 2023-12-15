from pathlib import Path
from pprint import pprint
import based58
from anchorpy import Program
from anchorpy_core.idl import Idl
from solana.rpc.api import Client
from solders.pubkey import Pubkey
from solders.signature import Signature

from decimal import Decimal


def wrapped_i80f48_to_float(wrapped_i80f48_value: int):
    nb_of_fractional_bits = 48
    value = Decimal(wrapped_i80f48_value)
    value = value / 2 ** nb_of_fractional_bits
    return value


print(wrapped_i80f48_to_float(-54953626681867088))

sample_logs = ["Program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL invoke [1]", "Program log: CreateIdempotent",
               "Program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL consumed 7338 of 400000 compute units",
               "Program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL success",
               "Program A7vUDErNPCTt9qrB6SSM4F6GkxzUe9d8P3cXSmRg4eY4 invoke [1]",
               "Program log: Instruction: LendingAccountWithdraw",
               "Program log: Withdrawing all: 1000000 of F9jRT1xL7PCRepBuey5cQG5vWHFSbnvdWxJWKqtzMDsd in GhV6ZftLXv3o38CHMhX6nu8GkxS3kvrHSSCVpGFTysUC",
               "Program log: withdraw_spl_transfer: amount: 1000000 from J9SAzLYETfcXBdrvswRaNUiGaMtmLiucwEJKEFW8d3FA to 4U3UNQU7spMKzY1cUviRdj9zAT2cVbGQEzioey1mCCZM, auth Fx99GAAXXk43peMfHxS2S7xTubazffA5h7ftmTEJK2bk",
               "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]", "Program log: Instruction: Transfer",
               "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4645 of 315437 compute units",
               "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
               "Program data: A9yU8yH5NlgFAAAAMC4xLjAF3hNv9VErtBxgns6TItDZs+IZ/Y6TNWGuTe16nnh4ye4CH6/bWLrtvuQMibfybmkmRSqFhHygh92ElC0GqUlcO8hKNv2sSk0p0XfYfozLqjspIR2sHsbx0eDvsV5tGY7pPk3uIRtapASqm9ALTdv++zxMXXSinbwKl99MTnuDkdJAOsgK1DjHp0u5LCmsGj4g7ioWbEEtiXn/B9aQ3U+4QEIPAAAAAAAB",
               "Program log: Expecting 0 remaining accounts", "Program log: Got 0 remaining accounts",
               "Program log: check_health: assets 0 - liabs: 0",
               "Program A7vUDErNPCTt9qrB6SSM4F6GkxzUe9d8P3cXSmRg4eY4 consumed 88164 of 392662 compute units",
               "Program A7vUDErNPCTt9qrB6SSM4F6GkxzUe9d8P3cXSmRg4eY4 success"]

sample_inner_ixs = [
    {"index": 1, "instructions": [{"programIdIndex": 9, "accounts": [5, 1, 4],
                                   "data": "11115hc6izQ5YEyLuqy666n8aGeujSQofY7ibwnRv64oCXeYpCg8t6ZaiuSwwbw6ev76Ut"}]}]

sample_message = "gAEABgwF3hNv9VErtBxgns6TItDZs+IZ/Y6TNWGuTe16nnh4yTOAiZ9axygRNRuzZCiQK4q5zvPfaH0CnR4tEUdeu2zY7gIfr9tYuu2+5AyJt/JuaSZFKoWEfKCH3YSULQapSVzpPk3uIRtapASqm9ALTdv++zxMXXSinbwKl99MTnuDkd4jxKWJrU3oI9SPceEA9wtTBJ5T78IpGTiq1XRzIrqD/r/JWnhhuWaJpdf6l+yCA38eN/l9xqMSjBXsYMAOea2MlyWPTiSJ8bs9ECkUjg2DC1oTmdr/EIQEjnvY2+n4WdJAOsgK1DjHp0u5LCmsGj4g7ioWbEEtiXn/B9aQ3U+4AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAG3fbh12Whk9nL4UbO63msHLSF7V9bN5E6jPWFfv8AqYd/H/1F6tUC+2l0WY9HS4WATM+2LA2i9Tq46cjeMS2lO8hKNv2sSk0p0XfYfozLqjspIR2sHsbx0eDvsV5tGY4/gcRh2keI2GQribtQSPXEZxYR1wZZ93XFhp/+f3fIFAIGBgABAAcICQEBCggLAgADAQQFCRIkSEoT0tLAwEBCDwAAAAAAAQEA"

http_client = Client("https://api.devnet.solana.com")
tx = http_client.get_transaction(
    Signature.from_string("4WdMP6s8SasqWrWpscurrN1r8ueLZNSdq81nLRJfyhsDU7xMz47t8xYh4hTvTy2C2sZ8Frc3x8urjfkYwKTg7dNT"),
    max_supported_transaction_version=0)

message = tx.value.transaction.transaction.message
pprint(message)
ix_data = message.instructions[0].data
# message_bytes = base64.b64decode(cpi.data)
# message_decoded = MessageV0.from_bytes(message_bytes[1:])

# merged_instructions = merge_instructions_and_cpis(message_decoded.instructions, sample_inner_ixs)
# expanded_instructions = expand_instructions(message_decoded.account_keys, merged_instructions)
# ix_with_logs = reconcile_instruction_logs("zrozUUvTujLSCxyT7JHtX48V5MYgkdQi4FAh5HreaH7p8n93bCDCo1huJsVBYiBkNXvFij7QgqYFC5jRRcXxpzi", expanded_instructions, sample_logs)

path = Path("idls/devnet/marginfi-v2.json")
raw = path.read_text()
idl = Idl.from_json(raw)

program = Program(idl, Pubkey.from_string("A7vUDErNPCTt9qrB6SSM4F6GkxzUe9d8P3cXSmRg4eY4"))
print(ix_data)
# message_bytes = based58.b58decode(b"CkHi86f1tcCVWQEX8TFsY7AwRnzu5ZQsh8Jnva7t4euknF6qyvtJuFDkiNsaraQBtLZcUNRCPJhAwexoH9EbTGFGi6uWVukVidkGz3LTaeevMSN6uqj5xjvVrDZVA5buqKQz86uErcvki7RNvez7QeoaBc19PV4YcLSfHkZpWUGxCCW87cfsShqCJgrsdjW45gkfKxqxb7t8T8FwVpudj2v7hZJzBqHSVvnQKWHW5ENmHHbYfSAFUGjQUhct6AnAWsoJ5XWoAUrm8G3ppN3fJe8vMGCvPfEnz76ea9LYQDeYSCqKg9f8QwZ2jj8z7xNfBUJw3MJh2hSxWxL635Hrx2xKnRpFT7vNugx2fpwwasGfBYkfMivjVfTVKcjJWSK46NXzmqhKX16ct7vpeBasue8eUM9hAtG5KDqs8XXQ3QkqeChm6qX2GJ7ohY6TRZAoQid767qLY84ZHEutqUibTQCtRUT15hHwbRRBAmAStWeBKJDopDUyHvxRXMxsG7dddT4pqNEukFRNu1chj4Sn2k2D8j9gehTESuKxtD6KVKBD3zb1MwpXGJK41TkUtTrDfn81REFmcAATB5srpXdfVig9XFDkZa2CUNrXwmmgMwR2LNx5Fv1mUtXwr93cvULv1VeXc5Qmfy66LK1mDaFVtR5iGqKAD4Tjf7vwJTA4i1Q1EFHrcZfFS3YT3QqVbbnjDGj59rnDtLqoWqzTLMSdnekbLnU98rndUgc8XUL3EX")
message_bytes = based58.b58decode(str.encode(str(ix_data)))
parsed = program.coder.instruction.parse(message_bytes)
print(parsed)

# for ix in ix_with_logs:
#     print("\n\n=============================================")
#     print("pid:", ix.message.program_id)
#     # print(log)
#     if ix.message.program_id == program.program_id:
#         parsed = program.coder.instruction.parse(ix.message.data)
#         print(parsed)
#
#     for inner_ix in ix.inner_instructions:
#         print("\n CPI  <<<<")
#         print("pid:", inner_ix.message.program_id)
#         if inner_ix.message.program_id == program.program_id:
#             parsed = program.coder.instruction.parse(inner_ix.message.data)
#             print(parsed)

# events_coder: EventCoder = EventCoder(idl)
#
# parser = EventParser(program.program_id, program.coder)
# events = []
# parser.parse_logs(sample_log, lambda evt: print(evt))

# event = MarginfiAccountCreateEvent(header=AccountEventHeader(
#     version="0.1.0",
#     signer=Pubkey.from_string("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
#     marginfi_account=Pubkey.from_string("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
#     marginfi_group=Pubkey.from_string("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
# ))
#
# pprint.pprint(str(event.__dataclass_fields__))

# test = bytes([3, 3, 3])
# test1 = base64.b64encode(test)
# test2 = base64.b64decode(test1)
# print(test2)
#
# ix_coder = program.coder.instruction
# message = ""
# sample_message_bytes = base64.b64decode(message)
# print(sample_message_bytes)
# sample_message = MessageV0.from_bytes(sample_message_bytes[1:])
# print(sample_message)
# message_decoded = ix_coder.parse(bytes)
