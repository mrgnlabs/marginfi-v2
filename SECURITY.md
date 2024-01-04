# Important Notice
**DO NOT CREATE A GITHUB ISSUE** to report a security problem. Instead, please send an email to security@mrgn.group with a detailed description of the attack vector and security risk you have identified.
​
# Bug Bounty Overview
marginfi offers bug bounties for marginfi's on-chain program code. UI-only bugs are omitted.
​
|Severity|Bounty|
|-----------|-------------|
|Critical|10% of the value of the hack, up to $500,000|
|High|$10,000 to $50,000 per bug, assessed on a case by case basis|
|Medium/Low|$1,000 to $5,000 per bug, assessed on a case by case basis|
​

The severity scale is based on [Immunefi's classification system](https://immunefi.com/immunefi-vulnerability-severity-classification-system-v2-3/). 
Note that these are simply guidelines for the severity of the bugs. Each bug bounty submission will be evaluated on a case-by-case basis.
​
## Submission
Please email security@mrgn.group with a detailed description of the attack vector.
​
For critical- and high-severity bugs, we require a proof of concept reproducible on a privately deployed mainnet contract (**NOT** our official deployment).
​
You should expect a reply within 1 business day with additional questions or next steps regarding the bug bounty.
​
## Bug Bounty Payment
Bug bounties will be paid in USDC or equivalent.
​
## Invalid Bug Bounties
A number of attacks are out of scope for the bug bounty, including but not limited to:
1. Attacks that the reporter has already exploited themselves, leading to damage.
2. Attacks requiring access to leaked keys/credentials.
3. Attacks requiring access to privileged addresses (governance, admin).
4. Incorrect data supplied by third party oracles (this does not exclude oracle manipulation/flash loan attacks).
5. Lack of liquidity.
6. Third party, off-chain bot errors (for instance bugs with an arbitrage bot running on the smart contracts).
7. Best practice critiques.
8. Sybil attacks.
9. Attempted phishing or other social engineering attacks involving marginfi contributors or users
10. Denial of service, or automated testing of services that generate significant traffic.
