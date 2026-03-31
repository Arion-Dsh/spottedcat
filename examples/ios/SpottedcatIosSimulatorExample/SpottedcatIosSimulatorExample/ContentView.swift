import SwiftUI

struct ContentView: View {
    @State private var started = false

    var body: some View {
        VStack(spacing: 12) {
            Text("Spottedcat iOS Simulator Example")
                .font(.headline)
            Text(started ? "Engine started" : "Starting...")
                .font(.subheadline)
        }
        .padding()
        .onAppear {
            guard !started else { return }
            started = true
            DispatchQueue.main.async {
                spottedcat_ios_start()
            }
        }
    }
}
