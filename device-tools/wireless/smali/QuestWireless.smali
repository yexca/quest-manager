# First-party shell helper. Resolves platform methods by name, never Binder IDs.
# No APK is installed. Inputs arrive only through the authorized USB stdin.
.class public LQuestWireless;
.super Ljava/lang/Object;

.method public static main([Ljava/lang/String;)V
    .locals 10
    :try_start
    const-string v0, "adb"
    invoke-static {v0}, Landroid/os/ServiceManager;->getService(Ljava/lang/String;)Landroid/os/IBinder;
    move-result-object v0
    invoke-static {v0}, Landroid/debug/IAdbManager$Stub;->asInterface(Landroid/os/IBinder;)Landroid/debug/IAdbManager;
    move-result-object v0
    const/4 v1, 0x0
    aget-object v2, p0, v1
    const-string v3, "stop"
    invoke-virtual {v3, v2}, Ljava/lang/String;->equals(Ljava/lang/Object;)Z
    move-result v2
    if-eqz v2, :start_pairing
    invoke-interface {v0}, Landroid/debug/IAdbManager;->disablePairing()V
    return-void

    :start_pairing
    new-instance v2, Ljava/io/BufferedReader;
    new-instance v3, Ljava/io/InputStreamReader;
    sget-object v4, Ljava/lang/System;->in:Ljava/io/InputStream;
    const-string v5, "UTF-8"
    invoke-direct {v3, v4, v5}, Ljava/io/InputStreamReader;-><init>(Ljava/io/InputStream;Ljava/lang/String;)V
    invoke-direct {v2, v3}, Ljava/io/BufferedReader;-><init>(Ljava/io/Reader;)V
    invoke-virtual {v2}, Ljava/io/BufferedReader;->readLine()Ljava/lang/String;
    move-result-object v3
    invoke-virtual {v2}, Ljava/io/BufferedReader;->readLine()Ljava/lang/String;
    move-result-object v4
    invoke-virtual {v2}, Ljava/io/BufferedReader;->readLine()Ljava/lang/String;
    move-result-object v5
    if-eqz v3, :invalid
    if-eqz v4, :invalid
    if-eqz v5, :invalid
    const-string v6, "(?i)[0-9a-f]{2}(:[0-9a-f]{2}){5}"
    invoke-virtual {v3, v6}, Ljava/lang/String;->matches(Ljava/lang/String;)Z
    move-result v6
    if-eqz v6, :invalid
    const-string v6, "studio-[0-9a-f]{16}"
    invoke-virtual {v4, v6}, Ljava/lang/String;->matches(Ljava/lang/String;)Z
    move-result v6
    if-eqz v6, :invalid
    const-string v6, "[0-9a-f]{32}"
    invoke-virtual {v5, v6}, Ljava/lang/String;->matches(Ljava/lang/String;)Z
    move-result v6
    if-eqz v6, :invalid
    invoke-interface {v0}, Landroid/debug/IAdbManager;->isAdbWifiSupported()Z
    move-result v6
    if-eqz v6, :invalid
    # Allow this network for this session; do not persist a trusted-network entry.
    invoke-interface {v0, v1, v3}, Landroid/debug/IAdbManager;->allowWirelessDebugging(ZLjava/lang/String;)V
    const/16 v6, 0x14
    :wait_port
    invoke-interface {v0}, Landroid/debug/IAdbManager;->getAdbWirelessPort()I
    move-result v7
    if-gtz v7, :port_ready
    const-wide/16 v8, 0x1f4
    invoke-static {v8, v9}, Ljava/lang/Thread;->sleep(J)V
    add-int/lit8 v6, v6, -0x1
    if-gtz v6, :wait_port
    goto :invalid
    :port_ready
    const v6, 0xffff
    if-gt v7, v6, :invalid
    # Let the parent try existing trust before opening a new pairing listener.
    sget-object v3, Ljava/lang/System;->out:Ljava/io/PrintStream;
    invoke-virtual {v3, v7}, Ljava/io/PrintStream;->println(I)V
    invoke-virtual {v3}, Ljava/io/PrintStream;->flush()V
    invoke-virtual {v2}, Ljava/io/BufferedReader;->readLine()Ljava/lang/String;
    move-result-object v6
    const-string v7, "pair"
    invoke-virtual {v7, v6}, Ljava/lang/String;->equals(Ljava/lang/Object;)Z
    move-result v6
    if-eqz v6, :done
    invoke-interface {v0, v4, v5}, Landroid/debug/IAdbManager;->enablePairingByQrCode(Ljava/lang/String;Ljava/lang/String;)V
    const-string v6, "pairing"
    invoke-virtual {v3, v6}, Ljava/io/PrintStream;->println(Ljava/lang/String;)V
    invoke-virtual {v3}, Ljava/io/PrintStream;->flush()V
    # Parent closes stdin after verification. The shell wrapper imposes a deadline.
    invoke-virtual {v2}, Ljava/io/BufferedReader;->readLine()Ljava/lang/String;
    invoke-interface {v0}, Landroid/debug/IAdbManager;->disablePairing()V
    :done
    :try_end
    return-void
    .catch Ljava/lang/Throwable; {:try_start .. :try_end} :failure
    :invalid
    const/4 v0, 0x1
    invoke-static {v0}, Ljava/lang/System;->exit(I)V
    return-void
    :failure
    move-exception v0
    # Platform exception messages can contain private inputs; return a fixed error.
    sget-object v0, Ljava/lang/System;->err:Ljava/io/PrintStream;
    const-string v1, "USB wireless setup is unavailable on this system."
    invoke-virtual {v0, v1}, Ljava/io/PrintStream;->println(Ljava/lang/String;)V
    const/4 v0, 0x1
    invoke-static {v0}, Ljava/lang/System;->exit(I)V
    return-void
.end method
