# How to contribute

### Content
- [Making Issues](#making-issues)
- [Code Changes](#code-changes)
	- [Consistency](#consistency)
	- [Quality and Substance](#quality-and-substance)
- [Commits](#commits)
	- [Scale and Scope](#scale-and-scope)
	- [Messages](#messages)
- [Pull Requests](#pull-requests)
- [Tests](#tests)
- [Licensing](#licensing)

# Making Issues
Before contributing any code, you should understand how we use issues and how to make them in a way which ensures you will get the response you want.

Issues are used by all repositories for most interactions. Issues are used for reporting actual issues with the codebase, as well as making feature requests and asking general questions. When making an issue, please use one of the provided issue templates. Using the provided templates keeps things consistent, as well as makes it easier for any of our 3rd party tools to interact with issues. If a template does not exist for what you wish to do, create a feature request for one. Not using an issue template may result in your issue being closed without completion.

When making code changes, an issue for the changes must be made and marked as approved before making the relevant changes. This ensures that your time is not wasted on changes we are not interested in making.

When making issues we ask that you be as detailed as possible. Do not make issues with titles like "I had an issue", "Found a bug", etc. The issue title should be an adequate summary of the information found within the issue's contents. When writing the issue body, provide as much detail as possible. Make the issue as long as it needs to be in order to adequately get the information across. We welcome the use of images, videos, etc. as well when applicable. Provide error codes, error messages, timestamps, the actions leading up errors, etc. Relevant links and even code snippets are also asked for when applicable. The more details we have from the start, the less time we spend asking clarifying questions resulting in faster resolutions. If you do not have much information, that is alright as well. We simply ask that you provide us with as much as you can from the start, and have patience as we work things out.

# Code Changes
As stated in [Making Issues](#making-issues), before making any code changes there must be an open, approved, issue for them. If you can not find an approved issue for the changes you wish to make, please make one before continuing.

There are 2 main goals when making code contributions:

1. [Consistency](#consistency)
2. [Quality/Substance](#quality-and-substance)

## Consistency
Arguably the most important thing about contributions is keeping them consistent with the rest of the codebase/project. With very few exceptions, contributions should be consistent with the existing codebases style, implementations/patterns, tech stack, etc. Doing so will ensure that anyone can jump into any repository and easily navigate about it. If you would like to make changes which go against any current consistency guidelines or implementations (such as changing linter rules, or swapping to a different tool in the stack), we ask that you make an issue specifically for these ideas first so they can be discussed by the core team.

See each repository for it's style and linting rules, however some higher level guidelines are:

- NEX (game) servers are written in Rust.
- Game servers which require databases use MySQL.
- All other servers (with few exceptions) are written in TypeScript with the intention of being run on Node. If a server is not written in TypeScript, it needs to be migrated to it. Runtimes besides Node (Bun, Deno, etc.) are not accounted for. If compatibility for another runtime can be added without introducing regressions when running under Node, and without significant refactoring, then support may be added via a pull request.
- Package managers besides npm are not accounted for. If compatibility for another package manager can be added without introducing regressions when running under npm, and without significant refactoring, then support may be added via a pull request.
- TypeScript servers which require databases use MongoDB.
- Given that our stacks are mostly Go and TypeScript, our tools and libraries are also written in Go and TypeScript depending on where they will be used. For desktop applications we typically prefer [Electron](https://electronjs.org/), as it allows us to reuse our existing libraries.

## Quality and Substance
We do not accept changes for the sake of changes. Changes should solve real problems, not change things for your personal preferences. This does not mean changes need to be *large*, however. A spelling error is a "real problem" and is worth changing, despite being a small change. However changes such as "Changed from `for...of` to `forEach`" will likely be rejected unless some additional problem is being solved with the change.

This does not mean we do not value the opinions of others, however. If you feel that a change should be made, but does not solve a specific problem (such as a refactoring change), we welcome opening a feature request for these changes. We do not claim to be infallible, and we are open to making stylistic changes to our codebases when they make sense. However changes like these must still be approved, and justified. If the changes do not provide any true substance, they will likely be rejected.

Requiring changes to be approved and having substance is essential to not wasting the time of both contributors (who may spend time making changes we are not interested in) and our developers (who will have to spend time reviewing changes which ultimately get rejected).

# Commits
Besides the changes themselves, commits are the most important part of contributions. There are 2 major things to keep in mind for commits:

1. [Scale/Scope](#scale-and-scope)
2. [Messages](#messages)

## Scale and Scope
The scale and scope of a commit should be reasonable. Do not commit for every line when making multiple changes, for example. However you should not include many unrelated changes in a single commit. By limiting the scope of the commit we can ensure that if any regressions or new bugs are introduced we can easily revert those changes without the need for major refactors or reimplementations. Limiting the scale of commits also makes review of the changes easier and faster.

## Messages
Commit messages should adequately explain the changes in the commit. Messages like "Updated file.md" and "spelling error" should not be used. Nonsense messages such as "oops" or "fixed" are especially not allowed. Commit messages should, at minimum, be in the format `type: message` where `type` represents the type or scope of the changes (`feat`, `chore`, `docs`, `fix`, etc.) and `message` is the actual changes. Unless the word is from the codebase and starts with a capital letter (such as an exported Go struct), the `message` should be lowercase. We also recommend using both "subject" and "body" commits. This can be achieved through the git CLI by using multiple `-m`/`--message` flags. For example `git commit -m "short subject" -m "longer description of the changes"`.

The following are examples of good commit messages:

- `chore: renamed nnid service to nnas`
- `fix: fixed hang in MutexMap.Has`

Please refer to [Conventional Commits](https://conventionalcommits.org/) for a detailed guide on how to structure commit messages. Writing good, detailed, commit messages helps ensure that we can refer back to the git history and quickly find where specific changes occurred in the event that they need further review, reverting, etc.

# Pull Requests
As stated in [Making Issues](#making-issues), before making a pull request there must be an open, approved, issue for the changes being made. If you can not find an approved issue for the changes you wish to make, please make one before continuing. Unless a single update closes multiple issues, each pull request should target a single issue. If you wish to work on multiple issues, please open a pull request for each. This keeps the pull request scope limited and allows for easier discussion of the individual issues and your related changes to them.

Before making a pull request ensure you have tested all changes and that no regressions have been introduced.

Pull requests should never be made against the default (`main`/`master`) branch of a repository. The default branch contains the most recent, stable, version of the codebase. All work on the codebase should take place in other branches. Unless otherwise specified, your target branch should typically be the `dev` branch. You may target other feature branches, however, if need be. If a `dev` branch does not exist for the repository you are working on, please submit a feature request for one to be added before continuing.

A pull request does not necessarily need to *close* an issue. A pull request may be made which implements only a subset of the requirements to close an issue, but does not fully complete the task itself. A pull request should never be *unfinished* code, however. All code must be tested and shippable. A pull request must at minimum bring an issue closer to closing without introducing any new regressions.

Like everything else, pull requests should be as detailed as possible. Your title should adequately summarize the changes being made, and the body of the pull request should fully explain your changes. We ask that, if applicable, the rationale behind your changes also be noted. For example rather than simply "Changed from `for...of` to `forEach`", if the change was made for a performance reason you should say "Changed from `for...of` to `forEach` due to `forEach` being X times faster in this case" and provide some benchmarks. Adding images, videos, etc. is also welcomed in order to illustrate changes. If the changes being made are directly tied to some form of visual (such as a change to the website, a tools GUI, etc.) then images or videos is ***REQUIRED***. If none are provided, then we may delay review until they are given. Providing visual examples of these changes allow us to quickly assess whether or not we wish to proceed with the changes being made.

If a pull request requires any database migrations, describe them in detail and leave any migration queries inside of a code block within a `<details>` tag. This should happen either at the very beginning of the pull request message, or at the very end, but not somewhere in between. Doing so makes it clear at a glance that there are migrations required and makes it easy to find the related queries.

We ask that you have patience with us as we review your pull request. SPFN only has a single full time developer, all other work is done by volunteers on their own time. Due to the sheer number of issues and pull requests, alongside our other general work and research, it may take us some time to fully review and decide on whether or not to merge your changes.

# Tests
We do not require 100% code coverage in any tests. We do not currently have strict rules regarding tests, however we may ask that tests be provided for large or complex changes.

# Licensing
Unless otherwise specified all code is licensed under [GNU AGPLv3](https://choosealicense.com/licenses/agpl-3.0), including that of outside contributions. This license allows users many freedoms to use our code in their own applications, including private and commercial use, while ensuring that all derivatives remain under this same license and keeps the source available, even when used over a network. A repository's license may not be changed by outside contributors unless that change is done with good reason, has been approved by the core development team, and is done with the consent of all relevant contributors.
