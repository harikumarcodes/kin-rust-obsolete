# Kin Rust (Obsolete)

Welcome!

This project was designed to allow developers to easily incorporate the Kin cryptocurrency into their Rust applications, as well as act as a bridge for SDKs written in higher level languages such as the Godot Kin SDK that I was developing. However, as I was working on it, I learned that the underlying service it uses, called Agora, will soon be replaced with a newer one called Kinetic. Because of this, I have decided to halt development on this project.

Although the project is not complete, I have decided to upload the code to this repository in case it may be useful to others or in case I or someone else may want to use it in the future to incorporate Kinetic and finish the project. I have implemented most of the desired functionality, including creating Solana accounts, creating Kin token accounts, merging token accounts, making payments, creating invoices and memos, and claiming airdrops in the test environment. The only things that are remaining are implementing async retry on failure and performing integration testing. I have run unit tests on the code, which all passed successfully. Please keep this in mind as you review the code.

Thank you for taking the time to view this project.

## License Information
This project is licensed under the MIT License. This means that you are free to use, modify, and distribute the code as you see fit, as long as you include the original copyright and license notice in any copies or modifications. Please see the included LICENSE file for more information.
