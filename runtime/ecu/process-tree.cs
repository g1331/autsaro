// Native Windows lifecycle for the offline verifier. No external kill utility
// is needed, and the command cannot run before it belongs to its private job.
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace Autosar
{
    public static class EcuProcessTree
    {
        [StructLayout(LayoutKind.Sequential)]
        private struct Security
        {
            public int Length;
            public IntPtr Descriptor;
            public int Inherit;
        }
        [StructLayout(LayoutKind.Sequential)]
        private struct Startup
        {
            public int Size;
            public IntPtr Reserved, Desktop, Title;
            public int X, Y, Width, Height, XChars, YChars, Fill, Flags;
            public short Show, ReservedSize;
            public IntPtr ReservedBytes, Input, Output, Error;
        }
        [StructLayout(LayoutKind.Sequential)]
        private struct StartupEx
        {
            public Startup Startup;
            public IntPtr Attributes;
        }
        [StructLayout(LayoutKind.Sequential)]
        private struct ProcessInfo
        {
            public IntPtr Process, Thread;
            public uint ProcessId, ThreadId;
        }
        [StructLayout(LayoutKind.Sequential)]
        private struct BasicLimits
        {
            public long ProcessTime, JobTime;
            public uint Flags;
            public UIntPtr Minimum, Maximum;
            public uint Active;
            public UIntPtr Affinity;
            public uint Priority, Scheduling;
        }
        [StructLayout(LayoutKind.Sequential)]
        private struct ExtendedLimits
        {
            public BasicLimits Basic;
            public ulong ReadOps, WriteOps, OtherOps, ReadBytes, WriteBytes, OtherBytes;
            public UIntPtr ProcessMemory, JobMemory, PeakProcess, PeakJob;
        }
        [StructLayout(LayoutKind.Sequential)]
        private struct Accounting
        {
            public long User, Kernel, PeriodUser, PeriodKernel;
            public uint Faults, Total, Active, Terminated;
        }
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr CreateJobObjectW(IntPtr attributes, string name);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool SetInformationJobObject(IntPtr job, int kind, ref ExtendedLimits limits, int size);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool TerminateJobObject(IntPtr job, uint code);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool QueryInformationJobObject(IntPtr job, int kind, out Accounting accounting, int size, IntPtr returned);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool CreateProcessW(string application, StringBuilder command, IntPtr processSecurity, IntPtr threadSecurity, bool inherit, uint flags, IntPtr environment, string directory, ref StartupEx startup, out ProcessInfo process);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool InitializeProcThreadAttributeList(IntPtr list, int count, int flags, ref IntPtr size);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool UpdateProcThreadAttribute(IntPtr list, uint flags, IntPtr attribute, IntPtr value, IntPtr size, IntPtr previous, IntPtr returned);
        [DllImport("kernel32.dll")]
        private static extern void DeleteProcThreadAttributeList(IntPtr list);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr CreateFileW(string path, uint access, uint sharing, ref Security security, uint disposition, uint attributes, IntPtr template);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern uint ResumeThread(IntPtr thread);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool GetExitCodeProcess(IntPtr process, out uint code);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool TerminateProcess(IntPtr process, uint code);
        [DllImport("kernel32.dll")]
        private static extern bool CloseHandle(IntPtr handle);

        public sealed class Result
        {
            public uint ExitCode;
            public string Stdout, Stderr;
        }
        private static void Check(bool ok, string operation)
        {
            if (!ok) throw new Win32Exception(Marshal.GetLastWin32Error(), operation);
        }
        private static IntPtr File(string path, uint access, uint disposition)
        {
            Security security = new Security();
            security.Length = Marshal.SizeOf(typeof(Security));
            security.Inherit = 1;
            IntPtr handle = CreateFileW(path, access, 3, ref security, disposition, 0x80, IntPtr.Zero);
            Check(handle != new IntPtr(-1), "Open command capture");
            return handle;
        }
        private static void StopTree(IntPtr job)
        {
            Check(TerminateJobObject(job, 1), "Terminate command tree");
            Stopwatch watch = Stopwatch.StartNew();
            while (true)
            {
                Accounting accounting;
                Check(QueryInformationJobObject(job, 1, out accounting, Marshal.SizeOf(typeof(Accounting)), IntPtr.Zero), "Observe command tree shutdown");
                if (accounting.Active == 0) return;
                if (watch.ElapsedMilliseconds >= 5000)
                    throw new InvalidOperationException("Command descendant shutdown could not be confirmed within 5s.");
                System.Threading.Thread.Sleep(10);
            }
        }
        public static Result Run(string scriptCommand, int timeoutSeconds)
        {
            if (timeoutSeconds < 1 || timeoutSeconds > Int32.MaxValue / 1000)
                throw new ArgumentOutOfRangeException("timeoutSeconds");
            string capture = Path.Combine(Path.GetTempPath(), "autosar-ecu-build-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(capture);
            string stdout = Path.Combine(capture, "stdout.txt");
            string stderr = Path.Combine(capture, "stderr.txt");
            IntPtr job = IntPtr.Zero, input = IntPtr.Zero, output = IntPtr.Zero, error = IntPtr.Zero;
            IntPtr attributes = IntPtr.Zero, handles = IntPtr.Zero;
            bool attributesReady = false;
            ProcessInfo process = new ProcessInfo();
            try
            {
                job = CreateJobObjectW(IntPtr.Zero, null);
                Check(job != IntPtr.Zero, "Create private job");
                ExtendedLimits limits = new ExtendedLimits();
                limits.Basic.Flags = 0x2000; // KILL_ON_JOB_CLOSE; no breakaway.
                Check(SetInformationJobObject(job, 9, ref limits, Marshal.SizeOf(typeof(ExtendedLimits))), "Configure private job");
                input = File("NUL", 0x80000000, 3);
                output = File(stdout, 0x40000000, 1);
                error = File(stderr, 0x40000000, 1);
                IntPtr attributeSize = IntPtr.Zero;
                InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref attributeSize);
                Check(attributeSize != IntPtr.Zero, "Size handle inheritance list");
                attributes = Marshal.AllocHGlobal(attributeSize);
                Check(InitializeProcThreadAttributeList(attributes, 1, 0, ref attributeSize), "Initialize handle inheritance list");
                attributesReady = true;
                handles = Marshal.AllocHGlobal(3 * IntPtr.Size);
                Marshal.WriteIntPtr(handles, 0, input);
                Marshal.WriteIntPtr(handles, IntPtr.Size, output);
                Marshal.WriteIntPtr(handles, 2 * IntPtr.Size, error);
                Check(UpdateProcThreadAttribute(attributes, 0, new IntPtr(0x20002), handles, new IntPtr(3 * IntPtr.Size), IntPtr.Zero, IntPtr.Zero), "Limit inherited command handles");
                StartupEx startup = new StartupEx();
                startup.Startup.Size = Marshal.SizeOf(typeof(StartupEx));
                startup.Startup.Flags = 0x100; // STARTF_USESTDHANDLES
                startup.Startup.Input = input;
                startup.Startup.Output = output;
                startup.Startup.Error = error;
                startup.Attributes = attributes;
                string powershell = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), @"WindowsPowerShell\v1.0\powershell.exe");
                string encoded = Convert.ToBase64String(Encoding.Unicode.GetBytes(scriptCommand));
                StringBuilder command = new StringBuilder("\"" + powershell + "\" -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand " + encoded);
                Check(CreateProcessW(powershell, command, IntPtr.Zero, IntPtr.Zero, true, 0x08080004, IntPtr.Zero, null, ref startup, out process), "Create suspended command");
                // The only user thread is still suspended: no descendant can
                // escape job ownership before assignment.
                Check(AssignProcessToJobObject(job, process.Process), "Assign suspended command");
                Check(ResumeThread(process.Thread) == 1, "Resume assigned command");
                uint waited = WaitForSingleObject(process.Process, (uint)(timeoutSeconds * 1000));
                if (waited == 258)
                {
                    StopTree(job);
                    throw new TimeoutException("Independent HostBatch build exceeded its host watchdog.");
                }
                Check(waited == 0, "Wait for command");
                uint code;
                Check(GetExitCodeProcess(process.Process, out code), "Read command exit code");
                StopTree(job);
                CloseHandle(output); output = IntPtr.Zero;
                CloseHandle(error); error = IntPtr.Zero;
                return new Result { ExitCode = code, Stdout = System.IO.File.ReadAllText(stdout), Stderr = System.IO.File.ReadAllText(stderr) };
            }
            finally
            {
                // Covers assignment, resume and observation failures as well
                // as timeout. Never resume a child if job assignment failed.
                if (job != IntPtr.Zero) { TerminateJobObject(job, 1); CloseHandle(job); }
                if (process.Process != IntPtr.Zero)
                {
                    TerminateProcess(process.Process, 1);
                    WaitForSingleObject(process.Process, 5000);
                    CloseHandle(process.Process);
                }
                if (process.Thread != IntPtr.Zero) CloseHandle(process.Thread);
                if (attributesReady) DeleteProcThreadAttributeList(attributes);
                if (attributes != IntPtr.Zero) Marshal.FreeHGlobal(attributes);
                if (handles != IntPtr.Zero) Marshal.FreeHGlobal(handles);
                if (input != IntPtr.Zero) CloseHandle(input);
                if (output != IntPtr.Zero) CloseHandle(output);
                if (error != IntPtr.Zero) CloseHandle(error);
                // Delete only the two files reserved above, never recursively
                // traverse a directory that another process could alter.
                try
                {
                    System.IO.File.Delete(stdout);
                    System.IO.File.Delete(stderr);
                    Directory.Delete(capture, false);
                }
                catch (IOException cleanup)
                {
                    Console.Error.WriteLine("Temporary ECU command captures retained at " + capture + ": " + cleanup.Message);
                }
                catch (UnauthorizedAccessException cleanup)
                {
                    Console.Error.WriteLine("Temporary ECU command captures retained at " + capture + ": " + cleanup.Message);
                }
            }
        }
    }
}
