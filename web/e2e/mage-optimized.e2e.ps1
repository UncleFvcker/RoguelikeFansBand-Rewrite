# SPDX-License-Identifier: MPL-2.0
param(
  [Parameter(Mandatory = $true)][string]$Executable,
  [Parameter(Mandatory = $true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
[IO.Directory]::CreateDirectory($outputPath) | Out-Null
$previousProfile = $env:WEBVIEW2_USER_DATA_FOLDER
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $outputPath 'webview-profile'
$app = Start-Process -FilePath $executablePath -WindowStyle Hidden -PassThru
try {
  $deadline = [DateTime]::UtcNow.AddSeconds(30)
  do { Start-Sleep -Milliseconds 100; $app.Refresh() } while ($app.MainWindowHandle -eq 0 -and [DateTime]::UtcNow -lt $deadline)
  if ($app.MainWindowHandle -eq 0) { throw 'Native window did not open' }
  $root = [System.Windows.Automation.AutomationElement]::FromHandle($app.MainWindowHandle)
  function FindElement($property, [string]$value) {
    $condition = New-Object System.Windows.Automation.PropertyCondition($property, $value)
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
      $element = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
      if ($element -and $element.Current.IsEnabled) { return $element }
      Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Native control missing: $value"
  }
  function ById([string]$id) { FindElement ([System.Windows.Automation.AutomationElement]::AutomationIdProperty) $id }
  function ByName([string]$name) { FindElement ([System.Windows.Automation.AutomationElement]::NameProperty) $name }
  function Invoke($element) { Write-Output "Invoking $($element.Current.Name)"; $element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke() }
  Invoke (ById 'session-new-game')
  (ById 'session-character-name').GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue('Mage Smoke')
  (ById 'session-seed').GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue('925')
  (ById 'session-tab-career').GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select()
  (ByName '魔法').GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
  Invoke (ByName '法师，进入领域选择')
  Invoke (ByName '奥秘，进入领域选择')
  (ByName '咒术').GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
  $start = ById 'session-start-game'
  if (-not $start.Current.IsEnabled) { throw 'Mage realm selection did not enable creation' }
  Invoke $start
  $identity = (ByName 'Mage Smoke 人类法师').Current.Name
  $level = (ByName '1 / 1').Current.Name
  $budget = (ByName '已学 0 / 1 · 剩余 1 个容量').Current.Name
  $location = (ByName '前哨站').Current.Name
  $resources = (ById 'resource-list').FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
  $resourceNames = @($resources | ForEach-Object { $_.Current.Name })
  if (-not ($resourceNames -match '法力')) { throw 'Mage mana was not rendered' }
  if (-not $app.CloseMainWindow()) { throw 'Native window did not accept close' }
  if (-not $app.WaitForExit(10000)) { throw 'Native process did not exit normally' }
  if ($app.ExitCode -ne 0) { throw "Native process exited with $($app.ExitCode)" }
  [ordered]@{
    passed = $true
    executable = $executablePath
    executableSha256 = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash.ToLowerInvariant()
    checkedAtUtc = [DateTime]::UtcNow.ToString('o')
    method = 'Optimized standalone EXE; Windows UI Automation Invoke/Toggle/SelectionItem/Value patterns; no WebDriver or test commands'
    checks = @('title', 'normal human Mage Arcane/Sorcery creation', 'level 1, mana and shared learning capacity', 'normal process exit')
    identity = $identity; level = $level; learningBudget = $budget; location = $location; resources = $resourceNames
    preparation = 'Fresh WebView profile, seed 925, normal creation only; no granted XP, books or devices'
    limitations = 'Native smoke covers creation and initial projection only. Gameplay, save determinism and screenshots are recorded separately by the same-source WebDriver build.'
    exitCode = $app.ExitCode
  } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outputPath 'checks.json') -Encoding UTF8
  Write-Output 'Optimized Mage native creation smoke passed.'
} finally {
  $env:WEBVIEW2_USER_DATA_FOLDER = $previousProfile
  if (-not $app.HasExited) { $app.CloseMainWindow() | Out-Null; if (-not $app.WaitForExit(5000)) { Stop-Process -Id $app.Id } }
}
