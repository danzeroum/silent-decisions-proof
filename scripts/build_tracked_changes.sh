#!/usr/bin/env bash
# Build a reviewer-facing tracked-changes PDF against the archived April source.
#
# Baseline provenance:
#   commit 61194cc is the final repository snapshot before the April submission;
#   its text matches the retained 10-page historical PDF on the central claims,
#   legal citations, equations, benchmark figures, and section structure.
#
# R1 (2026-09-15): compiles with tectonic when pdflatex is absent; stages the
# revised figure PDFs next to the flat sources so the diff compiles; output
# is named for the portal's "Main Document – Tracked Changes" slot.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
baseline="${TRACKED_BASELINE:-61194cc}"
work_dir="$(mktemp -d)"
out_dir="$repo_root/release"
output="$out_dir/COMSI-2026-04-0112_main_tracked_changes.pdf"
trap 'rm -rf "$work_dir"' EXIT

for tool in git tar latexpand latexdiff; do
  command -v "$tool" >/dev/null
done
if ! command -v pdflatex >/dev/null && ! command -v tectonic >/dev/null; then
  echo "no LaTeX engine found" >&2; exit 1
fi

git -C "$repo_root" cat-file -e "$baseline^{commit}"
mkdir -p "$work_dir/baseline" "$work_dir/current" "$out_dir"

git -C "$repo_root" archive "$baseline" paper1 | tar -x -C "$work_dir/baseline"
cp -a "$repo_root/paper1/." "$work_dir/current/"

(
  cd "$work_dir/baseline/paper1"
  latexpand main.tex > "$work_dir/baseline-flat.tex"
)
(
  cd "$work_dir/current"
  latexpand main.tex > "$work_dir/current-flat.tex"
)

latexdiff \
  --type=CFONT \
  --append-textcmd=href \
  "$work_dir/baseline-flat.tex" \
  "$work_dir/current-flat.tex" \
  > "$work_dir/tracked-changes.tex"

# Classic latexdiff-vs-tabular repair: the DIF wrapper tokens are robust
# (non-expandable) macros. When one directly precedes \hline or
# \begin{tabular}, TeX has already entered the alignment cell and reports
# "Misplaced \noalign". Dropping the wrapper token on those structural
# lines is safe — the begin/end pairs are otherwise inert markers, and the
# cell-level word markup is untouched.
sed -i -E 's/^\\DIF(add|del)endFL?[[:space:]]+(\\hline|\\begin\{tabular\})/\2/' \
  "$work_dir/tracked-changes.tex"

# Classic latexdiff brace repair: when a deleted passage's closing brace was
# moved into a "%DIFDELCMD < \MBLOCKRIGHTBRACE" comment while the
# corresponding "\DIFdel{" block from the PREVIOUS line is still open, the
# argument runs away to end-of-file. Restore the brace only in that case
# (blanket restoration over-closes other sites that the add-branch closes).
perl - "$work_dir/tracked-changes.tex" <<'PERLFIX'
use strict; use warnings;
my ($file) = @ARGV;
open(my $in, '<', $file) or die "open: $!";
my @lines = <$in>; close $in;
my $prev_unclosed_del = 0;
for my $i (0 .. $#lines) {
  my $line = $lines[$i];
  if ($line =~ /%DIFDELCMD < \\MBLOCKRIGHTBRACE/ && $prev_unclosed_del) {
    $line =~ s/(%DIFDELCMD < \\MBLOCKRIGHTBRACE)/}$1/;
    $lines[$i] = $line;
  }
  my $code = $line;
  $code =~ s/(?<!\\)%.*//;            # strip comments
  my $prev_code = $code;
  $prev_unclosed_del =
     ($prev_code =~ /\\DIFdel\{/ && $prev_code !~ /\\DIFdelend/ &&
      ($prev_code =~ tr/{//) > ($prev_code =~ tr/}//)) ? 1 : 0;
}
open(my $out, '>', $file) or die "write: $!";
print $out @lines; close $out;
PERLFIX

# Classic latexdiff deleted-label repair: latexdiff comments out \label{}
# commands that only exist in the old source ("%DIFDELCMD < \label{...}"),
# but any surviving \ref{...} — including refs inside struck-through
# \DIFdel text — then renders as "??" in the PDF. Restore exactly those
# commented labels that are still referenced, so deleted text keeps
# resolving (a renamed label such as tab:new -> tab:construction sits in
# the same float, so the restored label binds to the same table number).
# Unreferenced commented labels (e.g. sec:discussion inside a moved-
# subsection block guarded by \addtocounter) stay commented: restoring
# those would bind them to a decremented counter and risk duplicate
# hyperref anchors.
# Additionally, refs in old text may point at labels that never existed in
# the old source either (the April baseline shipped broken refs). The
# ALIAS map injects each missing label beside its semantic twin, so the
# struck-through ref resolves to the right section number.
perl - "$work_dir/tracked-changes.tex" <<'PERLFIX'
use strict; use warnings;
my ($file) = @ARGV;
my %alias = ('sec:linear_types' => 'sec:type_system');
open(my $in, '<', $file) or die "open: $!";
my @lines = <$in>; close $in;

my (%refs, %active, %commented);
for my $i (0 .. $#lines) {
  my $code = $lines[$i];
  $code =~ s/(?<!\\)%.*//;                      # strip comments
  $refs{$1}   = 1   while $code =~ /\\ref\{([^}]+)\}/g;
  $active{$1} = 1   while $code =~ /\\label\{([^}]+)\}/g;
  if ($lines[$i] =~ /%DIFDELCMD < \\label\{([^}]+)\}/) {
    $commented{$1}{$i} = 1;
  }
}
for my $name (keys %commented) {
  next unless $refs{$name} && !$active{$name};
  for my $i (keys %{$commented{$name}}) {
    $lines[$i] =~ s/%DIFDELCMD < \\label\{\Q$name\E\}/\\label{$name}/;
  }
  $active{$name} = 1;
}
for my $miss (keys %alias) {
  next unless $refs{$miss} && !$active{$miss};
  my $target = $alias{$miss};
  for my $i (0 .. $#lines) {
    my $code = $lines[$i];
    $code =~ s/(?<!\\)%.*//;
    if ($code =~ /\\label\{\Q$target\E\}/ && $code !~ /\\label\{\Q$miss\E\}/) {
      # NB: the \\label\{...\} braces must be escaped explicitly — a \\ inside
      # \Q..\E is TWO literal backslashes in Perl, so the \Q\\label{...}\E
      # form never matches a single-backslash \label command.
      $lines[$i] =~ s/(\\label\{\Q$target\E\})/$1\\label{$miss}/;
      $active{$miss} = 1;
      last;
    }
  }
  die "alias target \\label{$target} not found for $miss\n" unless $active{$miss};
}
open(my $out, '>', $file) or die "write: $!";
print $out @lines; close $out;
PERLFIX

cp "$repo_root/paper1/refs.bib" "$work_dir/refs.bib"
# R1: the revised sources \includegraphics{figures/...}; stage them for the
# flat-file compile (both engines resolve graphics relative to the CWD).
cp -a "$work_dir/current/figures" "$work_dir/figures"

(
  cd "$work_dir"
  if command -v pdflatex >/dev/null; then
    timeout 400 pdflatex -interaction=nonstopmode tracked-changes.tex >/dev/null
    timeout 300 bibtex tracked-changes >/dev/null
    timeout 400 pdflatex -interaction=nonstopmode tracked-changes.tex >/dev/null
    timeout 400 pdflatex -interaction=nonstopmode tracked-changes.tex >/dev/null
  else
    timeout 500 tectonic --keep-logs --keep-intermediates tracked-changes.tex >/dev/null 2>&1
  fi
)

test -s "$work_dir/tracked-changes.pdf"
cp "$work_dir/tracked-changes.pdf" "$output"
printf 'BUILD-TRACKED-CHANGES: OK — %s (baseline %s)\n' "$output" "$baseline"
