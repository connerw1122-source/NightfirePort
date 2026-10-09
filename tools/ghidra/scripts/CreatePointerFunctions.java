// Creates functions at code addresses that are only reached through pointers
// (function-pointer tables, lui/addiu address constants, callback arguments).
// Ghidra's default analysis leaves many of these as bare labels, which makes the
// recompiler miss them as indirect-call targets.
// @category PS2Recomp

import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

import java.util.TreeSet;

public class CreatePointerFunctions extends GhidraScript {

    private boolean isExecutable(Address a) {
        MemoryBlock b = currentProgram.getMemory().getBlock(a);
        return b != null && b.isExecute() && b.isInitialized();
    }

    private void consider(Address target, TreeSet<Address> out) {
        if (target == null || (target.getOffset() & 3) != 0 || !isExecutable(target))
            return;
        Function f = getFunctionContaining(target);
        if (f != null && f.getEntryPoint().equals(target))
            return; // already a function start
        if (f != null)
            return; // interior of a known function: a label, not a new entry
        out.add(target);
    }

    @Override
    public void run() throws Exception {
        TreeSet<Address> candidates = new TreeSet<>();

        // 1) Non-flow references (data/param/read) that land on executable code.
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferenceIterator(currentProgram.getMinAddress());
        while (refs.hasNext()) {
            Reference r = refs.next();
            if (r.getReferenceType().isFlow())
                continue;
            consider(r.getToAddress(), candidates);
        }

        // 2) Aligned 32-bit words in non-executable initialized blocks that point into code.
        for (MemoryBlock b : currentProgram.getMemory().getBlocks()) {
            if (!b.isInitialized() || b.isExecute())
                continue;
            Address a = b.getStart();
            while (a.compareTo(b.getEnd()) < 0) {
                if (monitor.isCancelled())
                    return;
                try {
                    long v = currentProgram.getMemory().getInt(a) & 0xFFFFFFFFL;
                    Address t = toAddr(v);
                    consider(t, candidates);
                } catch (Exception ignored) {
                }
                a = a.add(4);
            }
        }

        int created = 0;
        for (Address t : candidates) {
            if (getFunctionContaining(t) != null)
                continue;
            Instruction ins = getInstructionAt(t);
            if (ins == null) {
                DisassembleCommand cmd = new DisassembleCommand(new AddressSet(t), null, true);
                cmd.applyTo(currentProgram, monitor);
                ins = getInstructionAt(t);
            }
            if (ins == null)
                continue;
            if (createFunction(t, null) != null)
                created++;
        }
        println("CreatePointerFunctions: candidates=" + candidates.size() + " created=" + created);
    }
}
