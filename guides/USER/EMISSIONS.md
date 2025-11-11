# Emissions/Incentives

Want to learn more about how emissions and incentives work? Read on!

## Summary

Some banks, including banks from other venues, might offer a token incentive to depositors, or even
to borrowers. We call a set of incentives over time a "Campaign".

For example, a Campaign might distribute 7 tokens of A to lenders per week (one per day). Each
lender's share is determined on a pro-rata basis in real time. If there are two lenders, each
depositing the same amount, then each will be 3.5 tokens per week.

Now let's say there are two users, the first one has \$1 in deposits. User 2 deposits \$1 on
Thursday, and \$5 more on Saturday. This means User 1 and 2 both get 0.5 tokens/day on Thursday and
Friday. On Saturday and beyond, User 1 gets $1/(1+6)= 0.143$ tokens, and User 2 gets $6/(1+6)=0.857$
tokens/day.

Emissions/incentives are delivered by airdrop to the Account's authority, typically on Wednesday, in
no particular order. In the above example, User 1 would get $0.5 + 0.5 * 0.143 * 5 = 1.715$ tokens
and User 2 would get $0.5 + 0.5 + 0.857 * 5 = 5.285$ tokens

## Paired Emissions

In some Campaigns, users must BOTH lend a particular asset AND borrow a particular asset. A common
campaign is to lend some LST and borrow SOL. An Account only earns an airdrop if they perform both
tasks. For example, if the Campaign is to lend LST_A and borrow SOL, and a user is lending \$70 in A
and borrowing \$50 in SOL, they will earn rewards on \$50, i.e. min(lending_lst_a, borrowing_sol).

## Changing Emissions Destination

Want your emissions delivered somewhere specific? Set up an `emissions_destination_account` with
`marginfi_account_update_emissions_destination_account`.

This is highly recommended for PDA account authorities.


## FAQ

<details>
<summary> I didn't get any tokens, what's up?</summary>

We set a minimum threshold to get a drop in a given week, typically the value is just a little above
what it costs us to open an ATA (typically less than a dollar). If the airdrop you earned in a given
week is below that threshold, you might not get anything, though what you did earn will be
accumulated into the following week, where it will be dropped if you meet the threshold.

At the end of campaign, we typically do one final drop of any accumulated dust amounts, within
reason (don't try to spam accounts to get the ATA rent, it won't work!)

Contact support if you think you should have gotten a drop and didn't!
</details>

<details>
<summary> Do emissions always come in the same token as the Bank?</summary>

No, they could be in any asset, or even several!
</details>


<details>
<summary> Is there a set time emissions are dropped on Wednesday?</summary>

No, we vary the time slightly each week to avoid gamesmanship. It's usually during ET business
hours.
</details>

<details>
<summary> Can I earn multiple incentives at the same time?</summary>

Sure! If there is a paired incentive to lend LST_A or LST_B and borrow SOL, you can even count the
same SOL borrow towards both campaigns! For example, if you lend \$50 LST_A, and \$70 LST_B, then
borrow \$60 SOL, you would earn \$50 towards the LST_A Campaign and \$60 towards the LST_B Campaign.
</details>
