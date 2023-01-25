package cmd

import (
	"bytes"
	"fmt"
	"os/exec"
	"strings"

	"github.com/schollz/closestmatch"
)

// see https://github.com/schollz/closestmatch
var FUZZY_SEARCH_BAG_SIZES = []int{2, 3, 4}

func Filter(vs []string, f func(string) bool) []string {
	filtered := make([]string, 0)
	for _, v := range vs {
		if f(v) {
			filtered = append(filtered, v)
		}
	}
	return filtered
}

func get_all_system_test_targets() ([]string, error) {
	command := []string{"bazel", "query", "tests(//rs/tests:*)"}
	queryCmd := exec.Command(command[0], command[1:]...)
	outputBuffer := &bytes.Buffer{}
	stdErrBuffer := &bytes.Buffer{}
	queryCmd.Stdout = outputBuffer
	queryCmd.Stderr = stdErrBuffer
	if err := queryCmd.Run(); err != nil {
		return []string{}, fmt.Errorf("Bazel command: [%s] failed: %s", strings.Join(command, " "), stdErrBuffer.String())
	}
	cmdOutput := strings.Split(outputBuffer.String(), "\n")
	all_targets := Filter(cmdOutput, func(s string) bool {
		return len(s) > 0 && strings.Contains(s, "//rs/tests:")
	})
	return all_targets, nil
}

func get_closest_target_matches(target string) ([]string, error) {
	all_targets, err := get_all_system_test_targets()
	if err != nil {
		return []string{}, err
	}
	closest_matches := closestmatch.New(all_targets, FUZZY_SEARCH_BAG_SIZES).ClosestN(target, FUZZY_MATCHES_COUNT)
	return Filter(closest_matches, func(s string) bool {
		return len(s) > 0
	}), nil
}

func check_target_exists(target string) (bool, error) {
	command := []string{"bazel", "query", target}
	queryCmd := exec.Command(command[0], command[1:]...)
	stdErrBuffer := &bytes.Buffer{}
	queryCmd.Stderr = stdErrBuffer
	if err := queryCmd.Run(); err != nil {
		if strings.Contains(stdErrBuffer.String(), "no such target") {
			return false, nil
		} else {
			return false, fmt.Errorf("Bazel command: [%s] failed: %s", strings.Join(command, " "), stdErrBuffer.String())
		}
	}
	return true, nil
}
