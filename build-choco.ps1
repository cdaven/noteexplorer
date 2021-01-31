param([String]$version="0.0.0")

(Get-Content .\Cargo.toml) -Replace "version = ""[^""]+""", "version = ""$version""" | Set-Content .\Cargo.toml
(Get-Content .\Chocolatey\noteexplorer.nuspec) -Replace "<version>[^<]+</version>", "<version>$version</version>" | Set-Content .\Chocolatey\noteexplorer.nuspec

$target="x86_64-pc-windows-msvc"

cargo build --release --offline --target=$target
zip --junk-path R:\noteexplorer-win-x64-${version}.zip .\target\$target\release\noteexplorer.exe

Copy-Item .\target\$target\release\noteexplorer.exe .\Chocolatey\tools\
Set-Location .\Chocolatey
choco pack
Set-Location ..

""
"Version set to $version"
"When ready, publish package:"
"choco push .\Chocolatey\noteexplorer.$version.nupkg --source https://push.chocolatey.org/"
"Remember to update Changelog.md and commit"
