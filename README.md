# eSIM Mailer 📱💌

[![codecov](https://codecov.io/github/petersonmichaelj/esim-mailer/graph/badge.svg?token=CMFC13CPUO)](https://codecov.io/github/petersonmichaelj/esim-mailer)

eSIM Mailer is a cross-platform desktop application designed to simplify the process of sending eSIM activation details via email. It supports both Gmail and Outlook email providers and offers a user-friendly interface for composing and sending emails with eSIM QR codes.

## Why eSIM Mailer? 🤔

Distributing eSIM activation details can be a repetitive and time-consuming task, especially when dealing with a large number of eSIMs. eSIM Mailer streamlines this process by:

1. 🖥️ Providing a simple interface to input email and eSIM details
2. 📧 Supporting multiple email providers (Gmail and Outlook)
3. 🖼️ Allowing attachment of eSIM QR code images
4. 🔒 Handling OAuth2 authentication securely
5. 👀 Offering a preview of the email before sending

## The Story Behind eSIM Mailer 🍉

eSIM Mailer was born from a desire to expedite the process of sending eSIMs via [Connecting Humanity](https://connecting-humanity.org/), an initiative that connects Gazans with their families and the rest of the world. The repetitive nature of writing an email for each eSIM, typing out all the details every time, felt like a task ripe for automation. Recognizing that anything repetitive is easy to automate, this project was created to simplify and speed up the process of distributing eSIMs to Gazans. 🍉🌍❤️

## Download and Installation 💾

### Windows

1. Go to the [Releases](https://github.com/petersonmichaelj/esim-mailer/releases) page
2. Download the latest `esim-mailer-windows-x64.exe` file
3. Double-click the downloaded file to run the application

### macOS

1. Go to the [Releases](https://github.com/petersonmichaelj/esim-mailer/releases) page
2. Download the latest `esim-mailer-macos-x64.dmg` file for Intel Macs or `esim-mailer-macos-arm64.dmg` for Apple Silicon Macs
3. Open the downloaded .dmg file
4. Drag the eSIM Mailer app to your Applications folder
5. Open the app from your Applications folder

Note: On macOS, you may need to allow the app to run in System Preferences > Security & Privacy if you see a warning about the app being from an unidentified developer.

## Usage 🚀

1. Launch the eSIM Mailer application
2. Fill in the required fields:
   - From Email
   - To Email
   - BCC (optional)
   - Provider
   - Name
   - Data Amount
   - Time Period
   - Location
3. Click "Select Images" to choose the eSIM QR code image(s)
4. Review the email preview
5. Click "Send Email" to send the eSIM activation details

On first use, you'll be prompted to authorize the application with your email provider (Gmail or Outlook). Follow the on-screen instructions to complete the OAuth2 authentication process.

## Building from Source 🛠️

If you prefer to build the application from source, follow these steps:

1. Ensure you have Rust and Cargo installed on your system
2. Clone the repository: `git clone https://github.com/petersonmichaelj/esim-mailer.git`
3. Navigate to the project directory: `cd esim-mailer`
4. Build the application: `cargo build --release`
5. The built executable will be available in the `target/release` directory

## Contributing 🤝

Contributions are welcome! Please feel free to submit a Pull Request.

## License 📄

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
