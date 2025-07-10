<?php
if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $ip = $_POST['ip'] ?? '';
    $output = shell_exec("ping -c 2 " . $ip);
}
?>
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Enter a website to ping!</title>
</head>
<body>
    <h1>Ping an IP Address</h1>
    <form method="post">
        <label for="ip">IP Address:</label>
        <input type="text" id="ip" name="ip" required>
        <button type="submit">Ping</button>
    </form>
    <?php if (isset($output)): ?>
        <h2>Output:</h2>
        <pre><?php echo htmlspecialchars($output); ?></pre>
    <?php endif; ?>
</body>
</html>
